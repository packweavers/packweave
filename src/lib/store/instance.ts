import { api, pickFolder } from '../../api'
import type { Lockfile, LockedMod, SyncOp } from '../../types'
import { activeSource } from '../../types'
import { s, notify } from './state.svelte'
import { applyResolved } from './resolve'

function packSig(m: LockedMod): string {
	const src = activeSource(m)
	return [
		m.disabled ? 'off' : 'on',
		m.preferred,
		src?.versionId ?? '',
		src?.filename ?? '',
		src?.downloadUrl ?? '',
	].join('|')
}

function packDeltas(
	prev: Lockfile | null,
	next: Lockfile,
): { ids: Set<string>; files: Set<string> } {
	const before = new Map((prev?.mods ?? []).map((m) => [m.projectId, packSig(m)]))
	const ids = new Set<string>()
	const files = new Set<string>()
	for (const m of next.mods) {
		if (before.get(m.projectId) === packSig(m)) continue
		ids.add(m.projectId)
		const f = activeSource(m)?.filename
		if (f) files.add(f)
	}
	return { ids, files }
}

export async function loadBinding() {
	if (!s.pack) return
	s.sync = null
	try {
		s.instanceDir = await api.getBinding(s.pack.dir)
	} catch {
		s.instanceDir = null
	}
	if (s.instanceDir) void refreshSync()
}

export async function autoPushPackChanges(delta: { ids: Set<string>; files: Set<string> }) {
	if (!s.pack || !s.instanceDir || !s.sync) return
	const ops: SyncOp[] = []
	for (const m of s.sync.mods) {
		const pushable = m.kind === 'pack_only' || m.kind === 'version_diff' || m.kind === 'local_only'
		const isPackChange =
			(m.projectId != null && delta.ids.has(m.projectId)) ||
			(m.filename != null && delta.files.has(m.filename))
		if (pushable && isPackChange) {
			ops.push({
				target: 'mod',
				kind: m.kind,
				direction: 'push',
				projectId: m.projectId,
				slug: m.slug,
				provider: m.provider,
				name: m.name,
				instanceVersionId: m.instanceVersionId,
				filename: m.filename,
				relPath: m.relPath,
				projectType: m.projectType,
			})
		}
	}
	if (!ops.length) return
	const dir = s.pack.dir
	try {
		await api.applySync(dir, ops)
		await refreshSync(true)
	} catch {}
}

export async function autoSyncAfterEdit(prev: Lockfile | null, next: Lockfile, fromLoad: boolean) {
	if (!s.pack || !s.instanceDir) return
	await refreshSync(true)
	if (fromLoad || !s.autoPushOnSave) return
	await autoPushPackChanges(packDeltas(prev, next))
}

export async function linkInstance(instance: string) {
	if (!s.pack || !instance) return
	const dir = s.pack.dir
	try {
		await api.bindInstance(dir, instance)
		s.instanceDir = instance
		await refreshSync()
		notify('success', `Linked ${instance.split(/[/\\]/).pop()}`)
	} catch (e) {
		notify('error', `${e}`)
	}
}

export async function unlinkInstance() {
	if (!s.pack) return
	try {
		await api.unbindInstance(s.pack.dir)
	} catch (e) {
		notify('error', `${e}`)
	}
	s.instanceDir = null
	s.sync = null
}

export async function pullAllFromInstance() {
	if (!s.pack || !s.sync) return
	const ops: SyncOp[] = []
	if (s.sync.env) ops.push({ target: 'env', kind: 'env', direction: 'pull' })
	for (const m of s.sync.mods) {
		if (['instance_only', 'unknown', 'local_changed'].includes(m.kind)) {
			ops.push({
				target: 'mod',
				kind: m.kind,
				direction: 'pull',
				projectId: m.projectId,
				slug: m.slug,
				provider: m.provider,
				name: m.name,
				instanceVersionId: m.instanceVersionId,
				filename: m.filename,
				relPath: m.relPath,
				projectType: m.projectType,
			})
		}
	}
	for (const f of s.sync.files) {
		if (f.kind === 'new')
			ops.push({ target: 'file', kind: f.kind, direction: 'pull', path: f.path })
	}
	if (ops.length) await applySyncOps(ops)
}

export async function refreshSync(quiet = false) {
	if (!s.pack || !s.instanceDir) return
	if (s.scanBusy) {
		s.syncPending = true
		return
	}
	const dir = s.pack.dir
	const loud = !quiet || !s.sync
	s.scanBusy = true
	if (loud) s.scanning = true
	try {
		do {
			s.syncPending = false
			const report = await api.syncStatus(dir)
			if (s.pack?.dir !== dir || !s.instanceDir) return
			s.sync = report
		} while (s.syncPending && s.pack?.dir === dir && s.instanceDir)
	} catch (e) {
		notify('error', `${e}`)
	} finally {
		s.scanBusy = false
		s.scanning = false
	}
}

export async function applySyncOps(ops: SyncOp[]) {
	if (!s.pack || ops.length === 0) return
	const dir = s.pack.dir
	s.busy = true
	try {
		applyResolved(await api.applySync(dir, ops))
		await refreshSync()
		notify('success', `Applied ${ops.length} change${ops.length === 1 ? '' : 's'}`)
	} catch (e) {
		notify('error', `${e}`)
	} finally {
		s.busy = false
	}
}

export async function buildInstance() {
	if (!s.pack || !s.hasLock) return
	const packDir = s.pack.dir
	const dir = await pickFolder('Choose where to build the instance')
	if (!dir) return
	s.busy = true
	try {
		const report = await api.downloadMods(packDir, dir)
		if (report.failed.length === 0) {
			notify('success', `Downloaded ${report.downloaded} jars`)
		} else {
			notify(
				'warning',
				`Downloaded ${report.downloaded}, ${report.failed.length} failed: ${report.failed.join(', ')}`,
			)
		}
	} catch (e) {
		notify('error', `${e}`)
	} finally {
		s.busy = false
	}
}

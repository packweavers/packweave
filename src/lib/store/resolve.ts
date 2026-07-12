import { api } from '../../api'
import type { Lockfile, Manifest, PackResolved, VersionInfo } from '../../types'
import { s, notify } from './state.svelte'
import { refreshGit } from './git'
import { autoSyncAfterEdit } from './instance'

export function applyResolved(r: PackResolved) {
	if (s.pack) s.pack.manifest = r.manifest
	s.lockfile = r.lockfile
	s.validations = r.validations
	s.health = r.health
	void enrichAll()
	void refreshUnpublished()
	if (s.instanceDir) void autoSyncAfterEdit()
}

export function showState(lockfile: Lockfile | null) {
	s.lockfile = lockfile
	s.validations = []
	s.health = null
	void enrichAll()
	void refreshUnpublished()
}

export async function backgroundResolve() {
	if (!s.pack) return
	const dir = s.pack.dir
	try {
		const r = await api.resolvePack(dir, false)
		if (s.pack?.dir === dir) applyResolved(r)
	} catch {}
}

export async function refreshUnpublished() {
	if (!s.pack) return
	try {
		s.unpublished = await api.listUnpublished(s.pack.dir)
	} catch {
		s.unpublished = []
	}
}

export function clearEnrich() {
	s.versions = {}
	s.latest = {}
}

export function pickLatest(vs: VersionInfo[]): VersionInfo | null {
	return vs[0] ?? null
}

export async function enrichAll() {
	const lock = s.lockfile
	const dir = s.pack?.dir
	if (!lock || !dir) return
	const missingMeta = lock.mods
		.filter((m) => !s.meta[m.projectId])
		.map((m) => ({ id: m.projectId, provider: m.preferred }))
	s.enriching = true
	try {
		if (missingMeta.length) {
			try {
				const metas = await api.enrichMods(missingMeta)
				if (s.pack?.dir !== dir) return
				for (const m of metas) s.meta[m.projectId] = m
			} catch {}
		}
		for (const mod of lock.mods) {
			if (s.pack?.dir !== dir) return
			if (s.versions[mod.projectId]) continue
			try {
				const loader = mod.projectType === 'mod' ? s.loader : null
				const vs = await api.modVersions(mod.projectId, mod.preferred, s.minecraft, loader)
				s.versions[mod.projectId] = vs
				s.latest[mod.projectId] = pickLatest(vs)
			} catch {
				s.versions[mod.projectId] = []
				s.latest[mod.projectId] = null
			}
		}
	} finally {
		s.enriching = false
	}
}

export async function loadVersions(projectId: string, projectType: string, provider: string) {
	if (s.versions[projectId]) return
	try {
		const loader = projectType === 'mod' ? s.loader : null
		const vs = await api.modVersions(projectId, provider, s.minecraft, loader)
		s.versions[projectId] = vs
		s.latest[projectId] = pickLatest(vs)
	} catch {
		s.versions[projectId] = []
	}
}

export async function setModVersion(projectId: string, version: string) {
	if (!s.pack) return
	s.busy = true
	try {
		applyResolved(await api.setModVersion(s.pack.dir, projectId, version))
		refreshGit()
	} catch (e) {
		notify('error', `${e}`)
	} finally {
		s.busy = false
	}
}

export async function setModVersions(updates: { projectId: string; version: string }[]) {
	if (!s.pack || !updates.length) return
	s.busy = true
	try {
		applyResolved(await api.setModVersions(s.pack.dir, updates))
		refreshGit()
		notify('success', `Updated ${updates.length} ${updates.length === 1 ? 'mod' : 'mods'}`)
	} catch (e) {
		notify('error', `${e}`)
	} finally {
		s.busy = false
	}
}

export async function runResolve() {
	if (!s.pack) return
	s.busy = true
	try {
		applyResolved(await api.resolvePack(s.pack.dir, true))
	} catch (e) {
		notify('error', `${e}`)
	} finally {
		s.busy = false
	}
}

export async function updateSettings(
	minecraft: string,
	loader: string,
	loaderVersion: string | null,
	channel: string,
) {
	if (!s.pack) return
	const dir = s.pack.dir
	const manifest: Manifest = { ...s.pack.manifest, minecraft, loader, loaderVersion, channel }
	s.busy = true
	try {
		await api.saveManifest(dir, manifest)
		if (s.pack) s.pack.manifest = manifest
		clearEnrich()
		applyResolved(await api.resolvePack(dir))
	} catch (e) {
		notify('error', `${e}`)
	} finally {
		s.busy = false
	}
}

export async function setPackVersion(version: string) {
	if (!s.pack) return
	const v = version.trim() || '1.0.0'
	if (v === s.pack.manifest.version) return
	const manifest: Manifest = { ...s.pack.manifest, version: v }
	try {
		await api.saveManifest(s.pack.dir, manifest)
		if (s.pack) s.pack.manifest = manifest
	} catch (e) {
		notify('error', `${e}`)
	}
}

export async function applyPackIcon(source: string | null) {
	if (!s.pack) return
	s.busy = true
	try {
		if (source) await api.setPackIcon(s.pack.dir, source)
		else await api.clearPackIcon(s.pack.dir)
		await backgroundResolve()
		refreshGit()
		notify('success', source ? 'Updated pack icon' : 'Removed pack icon')
	} catch (e) {
		notify('error', `${e}`)
	} finally {
		s.busy = false
	}
}

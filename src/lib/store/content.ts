import { api } from '../../api'
import type { BulkCandidate, ConvertCandidate, DroppedFile, Fix, SearchHit } from '../../types'
import { s, notify } from './state.svelte'
import { applyResolved } from './resolve'

export async function addMod(
	hit: Pick<SearchHit, 'project_id' | 'slug' | 'title' | 'project_type'>,
) {
	if (!s.pack) return
	s.busy = true
	try {
		applyResolved(
			await api.addMod(s.pack.dir, hit.project_id, hit.slug, hit.title, hit.project_type),
		)
	} catch (e) {
		notify('error', `${e}`)
	} finally {
		s.busy = false
	}
}

export async function addContent(
	provider: string,
	item: {
		projectId: string
		slug: string
		name: string
		projectType: string
		pin?: string | null
		url?: string | null
	},
) {
	if (!s.pack) return
	s.busy = true
	try {
		applyResolved(
			await api.addContent(
				s.pack.dir,
				provider,
				item.projectId,
				item.slug,
				item.name,
				item.projectType,
				item.pin ?? null,
				item.url ?? null,
			),
		)
	} catch (e) {
		notify('error', `${e}`)
	} finally {
		s.busy = false
	}
}

export async function addDropped(items: DroppedFile[]) {
	if (!s.pack || !items.length) return
	s.busy = true
	try {
		applyResolved(await api.addDropped(s.pack.dir, items))
		const matched = items.filter((i) => i.matched && !i.alreadyInPack).length
		const bundled = items.filter((i) => !i.matched).length
		const parts = []
		if (matched) parts.push(`${matched} added`)
		if (bundled) parts.push(`${bundled} bundled as ${bundled === 1 ? 'override' : 'overrides'}`)
		if (parts.length) notify('success', parts.join(' · '))
	} catch (e) {
		notify('error', `${e}`)
	} finally {
		s.busy = false
	}
}

export async function bulkAdd(items: BulkCandidate[]) {
	if (!s.pack || !items.length) return
	s.busy = true
	try {
		applyResolved(await api.addContentBulk(s.pack.dir, items))
		notify('success', `Added ${items.length} ${items.length === 1 ? 'item' : 'items'}`)
	} catch (e) {
		notify('error', `${e}`)
	} finally {
		s.busy = false
	}
}

export function depSnapshot(): Map<string, string> {
	return new Map(
		(s.lockfile?.mods ?? [])
			.filter((m) => m.dependencyType === 'dependency')
			.map((m) => [m.projectId, m.name] as [string, string]),
	)
}

export function reportPrunedDeps(before: Map<string, string>, exclude: string[] = []) {
	const after = new Set(
		(s.lockfile?.mods ?? [])
			.filter((m) => m.dependencyType === 'dependency')
			.map((m) => m.projectId),
	)
	const ex = new Set(exclude)
	const pruned = [...before].filter(([id]) => !after.has(id) && !ex.has(id)).map(([, name]) => name)
	if (!pruned.length) return
	const shown = pruned.slice(0, 3).join(', ')
	const more = pruned.length > 3 ? ` +${pruned.length - 3} more` : ''
	notify(
		'info',
		`Pruned ${pruned.length} now-unused ${
			pruned.length === 1 ? 'dependency' : 'dependencies'
		}: ${shown}${more}`,
	)
}

export async function removeMod(projectId: string) {
	if (!s.pack) return
	const before = depSnapshot()
	s.busy = true
	try {
		applyResolved(await api.removeMod(s.pack.dir, projectId))
		reportPrunedDeps(before, [projectId])
	} catch (e) {
		notify('error', `${e}`)
	} finally {
		s.busy = false
	}
}

export async function promoteMod(projectId: string) {
	if (!s.pack) return
	s.busy = true
	try {
		applyResolved(await api.promoteMod(s.pack.dir, projectId))
		notify('success', 'Promoted to a standalone mod')
	} catch (e) {
		notify('error', `${e}`)
	} finally {
		s.busy = false
	}
}

export async function setDisabled(projectId: string, disabled: boolean) {
	if (!s.pack) return
	const before = depSnapshot()
	s.busy = true
	try {
		applyResolved(await api.setContentDisabled(s.pack.dir, projectId, disabled))
		reportPrunedDeps(before, [projectId])
	} catch (e) {
		notify('error', `${e}`)
	} finally {
		s.busy = false
	}
}

export async function bulkSetDisabled(keys: string[], disabled: boolean) {
	if (!s.pack || !keys.length) return
	const before = depSnapshot()
	s.busy = true
	try {
		applyResolved(await api.setContentDisabledBulk(s.pack.dir, keys, disabled))
		reportPrunedDeps(before, keys)
	} catch (e) {
		notify('error', `${e}`)
	} finally {
		s.busy = false
	}
}

export async function bulkRemove(keys: string[]) {
	if (!s.pack || !keys.length) return
	const before = depSnapshot()
	s.busy = true
	try {
		applyResolved(await api.removeModsBulk(s.pack.dir, keys))
		reportPrunedDeps(before, keys)
	} catch (e) {
		notify('error', `${e}`)
	} finally {
		s.busy = false
	}
}

export async function convertSearch(
	projectId: string,
	target: string,
): Promise<ConvertCandidate | null> {
	if (!s.pack) return null
	s.busy = true
	try {
		return await api.convertSearch(s.pack.dir, projectId, target)
	} catch {
		return null
	} finally {
		s.busy = false
	}
}

export async function convertLookup(
	projectId: string,
	target: string,
	query: string,
): Promise<ConvertCandidate | null> {
	if (!s.pack) return null
	s.busy = true
	try {
		return await api.convertLookup(s.pack.dir, projectId, target, query)
	} catch (e) {
		notify('error', `${e}`)
		return null
	} finally {
		s.busy = false
	}
}

export async function addAltSource(
	projectId: string,
	provider: string,
	candidate: ConvertCandidate,
) {
	if (!s.pack) return
	s.busy = true
	try {
		applyResolved(await api.addAltSource(s.pack.dir, projectId, provider, candidate.id))
		notify('success', `Added ${provider === 'curseforge' ? 'CurseForge' : 'Modrinth'} as a source`)
	} catch (e) {
		notify('error', `${e}`)
	} finally {
		s.busy = false
	}
}

export async function setPreferredSource(projectId: string, provider: string) {
	if (!s.pack) return
	s.busy = true
	try {
		applyResolved(await api.setPreferredSource(s.pack.dir, projectId, provider))
		notify(
			'success',
			`Preferred source is now ${provider === 'curseforge' ? 'CurseForge' : 'Modrinth'}`,
		)
	} catch (e) {
		notify('error', `${e}`)
	} finally {
		s.busy = false
	}
}

export async function removeAltSource(projectId: string, provider: string) {
	if (!s.pack) return
	s.busy = true
	try {
		applyResolved(await api.removeAltSource(s.pack.dir, projectId, provider))
	} catch (e) {
		notify('error', `${e}`)
	} finally {
		s.busy = false
	}
}

export async function applyFix(fix: Fix) {
	if (!s.pack) return
	if (fix.kind === 'add') {
		s.busy = true
		try {
			applyResolved(await api.addMod(s.pack.dir, fix.projectId, fix.slug, fix.name, 'mod'))
		} catch (e) {
			notify('error', `${e}`)
		} finally {
			s.busy = false
		}
	} else if (fix.kind === 'remove') {
		await removeMod(fix.projectId)
	}
}

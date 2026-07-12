import { api, openCurseforgePage, openModrinthPage } from '../api'
import { store } from './store.svelte'
import { activeSource } from '../types'
import type { LocalContent, LockedMod, VersionInfo } from '../types'

export function updateFor(mod: LockedMod): VersionInfo | null {
	if (!activeSource(mod)?.pin) return null
	const latest = store.latest[mod.projectId]
	if (!latest) return null
	const installedId = activeSource(mod)?.versionId
	if (!installedId || latest.id === installedId) return null
	const installed = (store.versions[mod.projectId] ?? []).find((v) => v.id === installedId)
	if (installed && latest.datePublished <= installed.datePublished) return null
	return latest
}

export function prerelease(v: VersionInfo | null | undefined): string | null {
	return v && v.versionType !== 'release' ? v.versionType : null
}

export function installedType(mod: LockedMod): string | null {
	const id = activeSource(mod)?.versionId
	return prerelease((store.versions[mod.projectId] ?? []).find((v) => v.id === id))
}

export function sideLabel(mod: LockedMod): string {
	const client = mod.clientSide !== 'unsupported'
	const server = mod.serverSide !== 'unsupported'
	if (client && !server) return 'client'
	if (!client && server) return 'server'
	return 'client + server'
}

export function sideValue(mod: LockedMod): string {
	const client = mod.clientSide !== 'unsupported'
	const server = mod.serverSide !== 'unsupported'
	if (client && !server) return 'client'
	if (!client && server) return 'server'
	return 'both'
}

export function channelOf(mod: LockedMod): string {
	return installedType(mod) ?? 'release'
}

export function providerLabel(p: string): string {
	if (p === 'url') return 'Direct URL'
	return store.providerName(p)
}

export interface ProvRow {
	provider: string
	preferred: boolean
	resolved: boolean
}

export function providersOf(mod: LockedMod): ProvRow[] {
	return Object.entries(mod.sources)
		.map(([provider, s]) => ({
			provider,
			preferred: provider === mod.preferred,
			resolved: !!s.downloadUrl,
		}))
		.sort((a, b) => (a.preferred === b.preferred ? 0 : a.preferred ? -1 : 1))
}

export function hasAlts(mod: LockedMod): boolean {
	return Object.keys(mod.sources).length > 1
}

export function addableProviders(mod: LockedMod): string[] {
	const present = new Set<string>(Object.keys(mod.sources))
	return store.enabledProviders.map((p) => p.id).filter((id) => !present.has(id))
}

export function openPage(mod: LockedMod) {
	if (mod.preferred === 'curseforge') openCurseforgePage(mod.slug, mod.projectType)
	else openModrinthPage(mod.slug)
}

export function onVersionOpen(mod: LockedMod) {
	store.loadVersions(mod.projectId, mod.projectType, mod.preferred)
}

export function setVersion(mod: LockedMod, versionId: string) {
	if (versionId === activeSource(mod)?.versionId) return
	store.setModVersion(mod.projectId, versionId)
}

export function unpubMissingDeps(u: LocalContent): string[] {
	const have = (id: string): boolean => {
		const k = id.toLowerCase()
		if (
			(store.lockfile?.mods ?? []).some(
				(m) => m.slug.toLowerCase() === k || m.projectId.toLowerCase() === k,
			)
		)
			return true
		return store.unpublished.some((x) => x.meta?.id?.toLowerCase() === k)
	}
	return (u.meta?.dependencies ?? [])
		.filter((d) => d.kind === 'required')
		.filter((d) => !have(d.id))
		.map((d) => d.id)
}

export async function removeUnpublished(relPath: string) {
	if (!store.pack) return
	try {
		await api.fsDelete(store.pack.dir, `overrides/${relPath}`)
		await store.refreshUnpublished()
		store.refreshGit()
	} catch (e) {
		store.notify('error', `${e}`)
	}
}

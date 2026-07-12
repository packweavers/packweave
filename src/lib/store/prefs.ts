import { api } from '../../api'
import type { Manifest } from '../../types'
import { s, type PackView, type RecentPack, type ThemePref } from './state.svelte'

export function initUi() {
	applyTheme()
	void loadProviders()
}

export async function loadProviders() {
	try {
		s.providers = await api.listProviders()
	} catch {
		s.providers = []
	}
}

export function applyTheme() {
	const root = document.documentElement
	root.classList.remove('theme-light', 'theme-dark')
	if (s.theme === 'light') root.classList.add('theme-light')
	else if (s.theme === 'dark') root.classList.add('theme-dark')
}

export function setTheme(theme: ThemePref) {
	s.theme = theme
	applyTheme()
	persistPrefs()
}

export function setAutoPush(value: boolean) {
	s.autoPushOnSave = value
	persistPrefs()
}

export function hydratePrefs(blob: Record<string, unknown>) {
	if (Array.isArray(blob.recents)) s.recents = blob.recents as RecentPack[]
	if (typeof blob.theme === 'string') s.theme = blob.theme as ThemePref
	if (typeof blob.autoPush === 'boolean') s.autoPushOnSave = blob.autoPush
	const rest = { ...blob }
	delete rest.recents
	delete rest.theme
	delete rest.autoPush
	s.prefs = rest
}

export async function loadPrefs() {
	try {
		hydratePrefs(JSON.parse(await api.readPrefs()) as Record<string, unknown>)
	} catch {}
}

export function persistPrefs() {
	const blob = {
		...s.prefs,
		recents: s.recents,
		theme: s.theme,
		autoPush: s.autoPushOnSave,
	}
	void api.writePrefs(JSON.stringify(blob))
}

export function getPref<T>(key: string, def: T): T {
	const v = s.prefs[key]
	return v === undefined || v === null ? def : (v as T)
}

export function setPref(key: string, value: unknown) {
	s.prefs[key] = value
	persistPrefs()
}

export function pushRecent(dir: string, manifest: Manifest) {
	const entry: RecentPack = {
		dir,
		name: manifest.name,
		minecraft: manifest.minecraft,
		loader: manifest.loader,
		lastOpened: Date.now(),
	}
	s.recents = [entry, ...s.recents.filter((r) => r.dir !== dir)].slice(0, 16)
	persistPrefs()
}

export function removeRecent(dir: string) {
	s.recents = s.recents.filter((r) => r.dir !== dir)
	persistPrefs()
}

export function resetView() {
	s.view = 'content'
	s.viewHistory = ['content']
	s.viewIndex = 0
}

export function setView(view: PackView) {
	if (s.view === view) return
	s.view = view
	s.viewHistory = s.viewHistory.slice(0, s.viewIndex + 1)
	s.viewHistory.push(view)
	s.viewIndex = s.viewHistory.length - 1
}

export function navBack() {
	if (s.viewIndex > 0) {
		s.viewIndex--
		s.view = s.viewHistory[s.viewIndex]
	}
}

export function navForward() {
	if (s.viewIndex < s.viewHistory.length - 1) {
		s.viewIndex++
		s.view = s.viewHistory[s.viewIndex]
	}
}

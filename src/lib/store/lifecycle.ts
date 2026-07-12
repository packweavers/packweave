import { api, pickFolder, pickImportFile, revealFolder } from '../../api'
import type { PackState } from '../../types'
import { s, notify } from './state.svelte'
import { applyResolved, backgroundResolve, clearEnrich, showState } from './resolve'
import { pushRecent, removeRecent, resetView } from './prefs'
import { loadBinding, pullAllFromInstance, refreshSync } from './instance'
import { dismissAuth, handleGitError, refreshGit } from './git'

export function closePack() {
	s.pack = null
	s.lockfile = null
	s.validations = []
	s.health = null
	s.instanceDir = null
	s.sync = null
	s.git = null
	s.busy = false
	s.scanning = false
	s.enriching = false
	dismissAuth()
	clearEnrich()
	resetView()
}

export async function openRecent(dir: string): Promise<boolean> {
	s.busy = true
	try {
		const state = await api.openPack(dir)
		s.pack = { dir: state.dir, manifest: state.manifest }
		clearEnrich()
		resetView()
		showState(state.lockfile)
		pushRecent(state.dir, state.manifest)
		void loadBinding()
		void refreshGit()
		void backgroundResolve()
		return true
	} catch (e) {
		notify('error', `${e}`)
		removeRecent(dir)
		return false
	} finally {
		s.busy = false
	}
}

export async function createPack(
	name: string,
	minecraft: string,
	loader: string,
	loaderVersion: string | null,
	instanceDir: string | null = null,
): Promise<boolean> {
	const parent = await pickFolder('Choose a folder for this pack')
	if (!parent) return false
	const safe =
		name
			.trim()
			.replace(/[^a-z0-9-_ ]/gi, '')
			.replace(/\s+/g, '-') || 'modpack'
	const dir = `${parent}/${safe}`
	s.busy = true
	try {
		const manifest = await api.createPack(dir, name.trim(), minecraft, loader, loaderVersion)
		s.pack = { dir, manifest }
		clearEnrich()
		resetView()
		applyResolved(await api.resolvePack(dir))
		if (instanceDir) {
			await api.bindInstance(dir, instanceDir)
			s.instanceDir = instanceDir
			await refreshSync()
			await pullAllFromInstance()
		} else {
			await loadBinding()
		}
		refreshGit()
		pushRecent(dir, manifest)
		notify('success', `Created “${manifest.name}”`)
		return true
	} catch (e) {
		notify('error', `${e}`)
		return false
	} finally {
		s.busy = false
	}
}

export async function openPack(): Promise<boolean> {
	const dir = await pickFolder('Open a pack folder')
	if (!dir) return false
	s.busy = true
	try {
		const state = await api.openPack(dir)
		s.pack = { dir: state.dir, manifest: state.manifest }
		clearEnrich()
		resetView()
		showState(state.lockfile)
		pushRecent(state.dir, state.manifest)
		notify('success', `Opened “${state.manifest.name}”`)
		void loadBinding()
		void refreshGit()
		void backgroundResolve()
		return true
	} catch (e) {
		notify('error', `${e}`)
		return false
	} finally {
		s.busy = false
	}
}

export async function importPack(): Promise<boolean> {
	const src = await pickImportFile()
	if (!src) return false
	return importPackFrom(src)
}

export async function importPackFrom(src: string): Promise<boolean> {
	const parent = await pickFolder('Choose a folder for the imported pack')
	if (!parent) return false
	s.busy = true
	try {
		const guess =
			src
				.split(/[/\\]/)
				.pop()
				?.replace(/\.(mrpack|zip)$/i, '') || 'imported-pack'
		const safe = guess.replace(/[^a-z0-9-_ ]/gi, '').replace(/\s+/g, '-') || 'imported-pack'
		const dest = `${parent}/${safe}`
		const state = await api.importPack(src, dest)
		s.pack = { dir: state.dir, manifest: state.manifest }
		clearEnrich()
		resetView()
		showState(state.lockfile)
		pushRecent(state.dir, state.manifest)
		notify('success', `Imported “${state.manifest.name}”`)
		void loadBinding()
		void refreshGit()
		void backgroundResolve()
		return true
	} catch (e) {
		notify('error', `${e}`)
		return false
	} finally {
		s.busy = false
	}
}

export function openedState(state: PackState) {
	s.pack = { dir: state.dir, manifest: state.manifest }
	clearEnrich()
	resetView()
	showState(state.lockfile)
	pushRecent(state.dir, state.manifest)
	void loadBinding()
	void refreshGit()
	void backgroundResolve()
}

export async function clonePack(url: string): Promise<boolean> {
	const u = url.trim()
	if (!u) return false
	const parent = await pickFolder('Choose where to clone the pack')
	if (!parent) return false
	return runClone(u, parent)
}

export async function runClone(url: string, parent: string): Promise<boolean> {
	s.busy = true
	try {
		const state = await api.clonePack(url, parent)
		openedState(state)
		notify('success', `Cloned “${state.manifest.name}”`)
		return true
	} catch (e) {
		await handleGitError(
			e,
			async () => {
				await runClone(url, parent)
			},
			url,
		)
		return false
	} finally {
		s.busy = false
	}
}

export async function reloadPack() {
	if (!s.pack) return
	const dir = s.pack.dir
	s.busy = true
	try {
		const state = await api.openPack(dir)
		s.pack = { dir, manifest: state.manifest }
		showState(state.lockfile)
		void refreshSync()
		void refreshGit()
		void backgroundResolve()
	} catch (e) {
		notify('error', `${e}`)
	} finally {
		s.busy = false
	}
}

export async function revealPackFolder() {
	if (!s.pack) return
	try {
		await revealFolder(s.pack.dir)
	} catch (e) {
		notify('error', `${e}`)
	}
}

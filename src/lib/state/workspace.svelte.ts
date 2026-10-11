import { emit, listen } from '@tauri-apps/api/event'
import { getCurrentWindow } from '@tauri-apps/api/window'

import { mainFolder } from '@/lib/tauri/git'
import { openPack, packIcon, pinLoaderVersion, saveManifest } from '@/lib/tauri/pack'
import { includePackweaver } from '@/lib/tauri/release'
import type { Manifest, TabPayload } from '@/lib/tauri/types'
import { attachWindow } from '@/lib/tauri/windows'
import { sameDir } from '@/lib/util/path'

import { app } from './app.svelte'
import { content } from './content.svelte'
import { files } from './files.svelte'
import { git } from './git.svelte'
import { instance } from './instance.svelte'
import { layout } from './layout.svelte'
import { preserve } from './preserve.svelte'
import { release } from './release.svelte'
import { selection } from './selection.svelte'
import { docId, isDocKind, tabs } from './tabs.svelte'
import { tasks } from './tasks.svelte'
import { toasts } from './toasts.svelte'

const APP_TITLE = 'packweave'
const PACK_EVENT = 'pack-changed'

export interface OpenOptions {
	detached?: boolean
	tab?: TabPayload | null
}

class Workspace {
	dir = $state<string | null>(null)
	mainDir = $state<string | null>(null)
	manifest = $state<Manifest | null>(null)
	icon = $state<string | null>(null)
	opening = $state(false)
	detached = $state(false)
	#refreshTimer: ReturnType<typeof setTimeout> | null = null
	#reloadTimer: ReturnType<typeof setTimeout> | null = null
	#pinTried = ''

	get isOpen(): boolean {
		return this.dir !== null && this.manifest !== null
	}

	get isMain(): boolean {
		return this.dir !== null && this.mainDir !== null && sameDir(this.dir, this.mainDir)
	}

	get name(): string {
		return this.manifest?.name ?? ''
	}

	get title(): string {
		if (!this.manifest) return APP_TITLE
		const branch = git.status?.branch
		return branch ? `${this.manifest.name} — ${branch}` : this.manifest.name
	}

	async open(dir: string, options: OpenOptions = {}): Promise<'opened' | 'elsewhere' | 'failed'> {
		this.opening = true
		try {
			const state = await openPack(dir)
			const elsewhere = await attachWindow(state.dir, state.manifest.name)
			if (elsewhere) return 'elsewhere'
			const mainDir = await mainFolder(state.dir).catch(
				tasks.or('Finding the main pack folder', state.dir),
			)
			this.#reset()
			this.dir = state.dir
			this.mainDir = mainDir
			this.manifest = state.manifest
			this.detached = options.detached === true
			layout.restore(state.dir, !this.detached)
			tabs.restore(state.dir, !this.detached)
			content.applyLockfile(state.lockfile)
			if (this.isMain && !this.detached) app.addRecent(state.dir, state.manifest)
			if (options.tab) this.receiveTab(options.tab)
			if (tabs.all.length === 0 && !this.detached) tabs.open('content', 'mod')
			void this.refreshIcon()
			void git.refreshAll().then(() => this.retitle())
			void instance.load()
			void content.warmUp()
			void preserve.load()
			this.ensureLoaderVersion()
			if (state.warning) tasks.fail('Upgrading the pack', state.warning)
			if (app.pack(mainDir).release.updates === 'url' && !this.detached) {
				void includePackweaver(state.dir)
					.then(() => this.changed())
					.catch(tasks.or('Adding packweaver to the pack', undefined))
			}
			return 'opened'
		} catch (error) {
			toasts.error(error)
			return 'failed'
		} finally {
			this.opening = false
		}
	}

	receiveTab(tab: TabPayload): void {
		if (!isDocKind(tab.kind)) return
		if (tab.buffer) files.adopt(tab.arg, tab.buffer.original, tab.buffer.text)
		tabs.open(tab.kind, tab.arg)
		if (tab.pinned) tabs.pin(docId(tab.kind, tab.arg), true)
	}

	follow(): () => void {
		const me = getCurrentWindow().label
		const stop = listen<{ dir: string; from: string }>(PACK_EVENT, ({ payload }) => {
			if (payload.from === me || !this.dir || !sameDir(payload.dir, this.dir)) return
			if (this.#reloadTimer) clearTimeout(this.#reloadTimer)
			this.#reloadTimer = setTimeout(() => void this.reload(false), 400)
		})
		return () => void stop.then((unlisten) => unlisten())
	}

	#announce(): void {
		if (this.dir) void emit(PACK_EVENT, { dir: this.dir, from: getCurrentWindow().label })
	}

	async reload(announce = true): Promise<void> {
		const dir = this.dir
		if (!dir) return
		try {
			const state = await openPack(dir)
			if (this.dir !== dir) return
			this.manifest = state.manifest
			content.applyLockfile(state.lockfile)
			if (state.warning) tasks.fail('Upgrading the pack', state.warning)
			void files.reloadClean()
			void this.refreshIcon()
			void instance.scan()
			void content.warmUp()
			await git.refreshAll()
			this.retitle()
			if (announce) this.#announce()
		} catch (error) {
			toasts.error(error)
		}
	}

	async close(): Promise<void> {
		this.#reset()
		await attachWindow(null, APP_TITLE)
	}

	retitle(): void {
		if (this.dir && !this.detached) void attachWindow(this.dir, this.title)
	}

	async saveManifest(manifest: Manifest): Promise<void> {
		if (!this.dir) return
		await saveManifest(this.dir, manifest)
		this.manifest = manifest
		if (this.isMain) app.addRecent(this.dir, manifest)
	}

	ensureLoaderVersion(): void {
		const dir = this.dir
		const manifest = this.manifest
		if (!dir || !manifest || this.detached) return
		if (manifest.loader === 'vanilla' || manifest.loaderVersion) return
		const attempt = `${dir}|${manifest.minecraft}|${manifest.loader}`
		if (attempt === this.#pinTried) return
		this.#pinTried = attempt
		void this.#pinLoaderVersion(dir)
	}

	async #pinLoaderVersion(dir: string): Promise<void> {
		try {
			const pinned = await pinLoaderVersion(dir)
			if (!pinned || this.dir !== dir) return
			this.manifest = pinned
			this.changed()
		} catch (error) {
			tasks.fail('Setting the loader version', error)
		}
	}

	async refreshIcon(): Promise<void> {
		if (!this.dir) return
		this.icon = await packIcon(this.dir).catch(tasks.or('Loading the pack icon', null))
	}

	changed(): void {
		if (this.#refreshTimer) clearTimeout(this.#refreshTimer)
		this.#refreshTimer = setTimeout(() => {
			void git.refresh()
			void content.refreshChanges()
			void content.refreshUnpublished()
			void preserve.load()
			instance.scheduleScan()
			this.#announce()
		}, 120)
	}

	#reset(): void {
		this.detached = false
		this.#pinTried = ''
		this.dir = null
		this.mainDir = null
		this.manifest = null
		this.icon = null
		content.reset()
		git.reset()
		instance.reset()
		files.reset()
		selection.reset()
		release.reset()
		tabs.reset()
	}
}

export const ws = new Workspace()

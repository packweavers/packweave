import { emit, listen } from '@tauri-apps/api/event'
import { getCurrentWindow } from '@tauri-apps/api/window'

import { appInfo, checkUpdate, installUpdate, readPrefs, writePrefs } from '@/lib/tauri/app'
import { listProviders } from '@/lib/tauri/catalog'
import type { AppInfo, Manifest, ProviderInfo, UpdateInfo } from '@/lib/tauri/types'
import { sameDir } from '@/lib/util/path'

import {
	defaultPackPrefs,
	defaultPrefs,
	parsePrefs,
	type PackPrefs,
	type Prefs,
	type ThemePref,
} from './prefs'
import { tasks } from './tasks.svelte'
import { toasts } from './toasts.svelte'

const PREFS_EVENT = 'prefs-changed'
const RECENT_LIMIT = 16

class AppState {
	prefs = $state<Prefs>(defaultPrefs())
	providers = $state<ProviderInfo[]>([])
	info = $state<AppInfo | null>(null)
	update = $state<UpdateInfo | null>(null)
	systemDark = $state(true)
	#saveTimer: ReturnType<typeof setTimeout> | null = null

	get theme(): 'light' | 'dark' {
		if (this.prefs.theme === 'system') return this.systemDark ? 'dark' : 'light'
		return this.prefs.theme
	}

	async load(): Promise<void> {
		const media = window.matchMedia('(prefers-color-scheme: dark)')
		this.systemDark = media.matches
		media.addEventListener('change', (e) => (this.systemDark = e.matches))
		try {
			this.prefs = parsePrefs(await readPrefs())
		} catch (error) {
			this.prefs = defaultPrefs()
			toasts.error(error)
		}
		const me = getCurrentWindow().label
		await listen<{ from: string; prefs: string }>(PREFS_EVENT, (event) => {
			if (event.payload.from !== me) this.prefs = parsePrefs(event.payload.prefs)
		})
		listProviders()
			.then((p) => (this.providers = p))
			.catch((error) => toasts.error(error))
		appInfo()
			.then((i) => (this.info = i))
			.catch((error) => toasts.error(error))
	}

	save(): void {
		if (this.#saveTimer) clearTimeout(this.#saveTimer)
		this.#saveTimer = setTimeout(() => {
			const json = JSON.stringify(this.prefs, null, 2)
			void writePrefs(json).catch((error) => toasts.error(error))
			void emit(PREFS_EVENT, { from: getCurrentWindow().label, prefs: json }).catch((error) =>
				tasks.fail('Sharing settings with other windows', error),
			)
		}, 150)
	}

	setTheme(theme: ThemePref): void {
		this.prefs.theme = theme
		this.save()
	}

	setScale(scale: number): void {
		this.prefs.uiScale = scale
		this.save()
	}

	setEditorSize(size: number): void {
		this.prefs.editorSize = size
		this.save()
	}

	pack(mainDir: string): PackPrefs {
		return this.prefs.packs[mainDir] ?? defaultPackPrefs()
	}

	updatePack(mainDir: string, change: (pack: PackPrefs) => void): void {
		const pack = $state.snapshot(this.pack(mainDir)) as PackPrefs
		change(pack)
		this.prefs.packs[mainDir] = pack
		this.save()
	}

	addRecent(dir: string, manifest: Manifest): void {
		const entry = {
			dir,
			name: manifest.name,
			minecraft: manifest.minecraft,
			loader: manifest.loader,
			lastOpened: Date.now(),
		}
		this.prefs.recents = [entry, ...this.prefs.recents.filter((r) => !sameDir(r.dir, dir))].slice(
			0,
			RECENT_LIMIT,
		)
		this.save()
	}

	removeRecent(dir: string): void {
		this.prefs.recents = this.prefs.recents.filter((r) => !sameDir(r.dir, dir))
		this.save()
	}

	providerReady(id: string): boolean {
		return this.providers.some((p) => p.id === id && p.configured)
	}

	async checkForUpdate(loud = false): Promise<UpdateInfo | null> {
		try {
			this.update = await checkUpdate()
		} catch (error) {
			this.update = null
			if (loud) toasts.error(error)
			else tasks.fail('Checking for a packweave update', error)
		}
		return this.update
	}

	installUpdate(): Promise<void> {
		return installUpdate()
	}
}

export const app = new AppState()

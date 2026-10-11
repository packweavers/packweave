import { settingKeys } from '@/lib/files/configform'
import { langOf } from '@/lib/files/highlight'
import { fileKind } from '@/lib/files/kind'
import { keyHints, pathHints, withFolders } from '@/lib/files/paths'
import * as fs from '@/lib/tauri/files'
import { autoPushFile } from '@/lib/tauri/instance'
import type { FsEntry } from '@/lib/tauri/types'
import { basename, dirname, join } from '@/lib/util/path'

import { app } from './app.svelte'
import { instance } from './instance.svelte'
import { tasks } from './tasks.svelte'
import { toasts } from './toasts.svelte'
import { ws } from './workspace.svelte'

export type BufferState = 'loading' | 'text' | 'binary' | 'large' | 'missing'

export interface FileBuffer {
	rel: string
	original: string
	text: string
	state: BufferState
	size: number
}

export const OVERRIDES = 'overrides'

export const PACK_FILES = [
	'.gitignore',
	'.packweaveignore',
	'.packweaveunlock',
	'modpack.json',
	'PACKWEAVE.md',
	'CHANGELOG.md',
]

export type FileSpot = 'shown' | 'found' | 'missing'

const HIDDEN_ROOT = ['index.json', '.pack-local.json', '.DS_Store', 'Thumbs.db', 'desktop.ini']

export function shownAtRoot(name: string): boolean {
	return !HIDDEN_ROOT.includes(name)
}

function mayHoldSettings(rel: string): boolean {
	return fileKind(rel) === 'config' || rel.toLowerCase().endsWith('.txt')
}

function shownInFiles(rel: string, isDir: boolean): boolean {
	if (rel.startsWith(`${OVERRIDES}/`)) return true
	return !isDir && !rel.includes('/') && shownAtRoot(rel)
}

class FilesState {
	buffers = $state<Record<string, FileBuffer>>({})
	listings = $state<Record<string, FsEntry[]>>({})
	expanded = $state<Record<string, boolean>>({})
	reveal = $state<{ rel: string; line: number; key?: string } | null>(null)
	focus = $state<{ rel: string } | null>(null)
	#index: Promise<string[]> | null = null

	get dirty(): string[] {
		return Object.values(this.buffers)
			.filter((b) => b.state === 'text' && b.text !== b.original)
			.map((b) => b.rel)
	}

	isDirty(rel: string): boolean {
		const buffer = this.buffers[rel]
		return !!buffer && buffer.state === 'text' && buffer.text !== buffer.original
	}

	reset(): void {
		this.reveal = null
		this.focus = null
		this.#index = null
		this.buffers = {}
		this.listings = {}
		this.expanded = {}
	}

	async list(dir: string): Promise<FsEntry[]> {
		const root = ws.dir
		if (!root) return []
		try {
			const entries = await fs.list(root, dir)
			if (ws.dir === root) this.listings[dir] = entries
			return entries
		} catch (error) {
			toasts.error(error)
			if (ws.dir === root) this.listings[dir] = []
			return []
		}
	}

	async refresh(): Promise<void> {
		this.#index = null
		await Promise.all(Object.keys(this.listings).map((dir) => this.list(dir)))
	}

	paths(): Promise<string[]> {
		const root = ws.dir
		if (!root) return Promise.resolve([])
		this.#index ??= fs.index(root).catch(tasks.or('Listing the pack files', []))
		return this.#index
	}

	async settingKeys(rel: string): Promise<string[]> {
		const root = ws.dir
		if (!root || !mayHoldSettings(rel)) return []
		const file = await fs.read(root, rel).catch(tasks.or(`Reading ${rel}`, null))
		if (!file?.text || fileKind(rel, file.text) !== 'config') return []
		return settingKeys(file.text, langOf(rel, file.text))
	}

	async packPaths(): Promise<string[]> {
		return withFolders(await this.paths())
	}

	async gamePaths(): Promise<string[]> {
		const prefix = `${OVERRIDES}/`
		const all = await this.paths()
		return withFolders(all.filter((p) => p.startsWith(prefix)).map((p) => p.slice(prefix.length)))
	}

	async editHints(text: string): Promise<string[]> {
		const bang = /^\s*!\s*/.exec(text) ? '!' : ''
		const rest = text.replace(/^\s*!\s*/, '').replace(/\\/g, '/')
		const hash = rest.indexOf('#')
		if (hash >= 0) {
			const file = rest.slice(0, hash).trim().replace(/^\/+/, '')
			const keys = await this.settingKeys(`${OVERRIDES}/${file}`)
			return keyHints(keys, rest.slice(hash + 1)).map((key) => `${bang}${file}#${key}`)
		}
		const typed = rest.trim().replace(/^\/+/, '')
		const paths = await this.gamePaths()
		if (paths.includes(typed) && mayHoldSettings(typed)) {
			const keys = await this.settingKeys(`${OVERRIDES}/${typed}`)
			if (keys.length > 0) return keys.slice(0, 8).map((key) => `${bang}${typed}#${key}`)
		}
		return pathHints(paths, typed).map((hint) => bang + hint)
	}

	async reloadClean(): Promise<void> {
		const clean = Object.keys(this.buffers).filter((rel) => !this.isDirty(rel))
		await Promise.all([...clean.map((rel) => this.reload(rel)), this.refresh()])
	}

	toggle(dir: string): void {
		this.expanded[dir] = !this.expanded[dir]
		if (this.expanded[dir] && !this.listings[dir]) void this.list(dir)
	}

	async locate(target: string, isDir: boolean): Promise<FileSpot> {
		const root = ws.dir
		const rel = target.replace(/\/+$/, '')
		if (!root || !rel) return 'missing'
		const parent = dirname(rel)
		const shown = shownInFiles(rel, isDir)
		const entries = shown
			? await this.list(parent)
			: await fs.list(root, parent).catch(tasks.or(`Listing ${parent || 'the pack'}`, []))
		if (!entries.some((e) => e.name === basename(rel) && e.isDir === isDir)) return 'missing'
		if (!shown) return 'found'
		const chain: string[] = []
		for (let dir = parent; dir; dir = dirname(dir)) chain.unshift(dir)
		await Promise.all(chain.filter((dir) => dir !== parent).map((dir) => this.list(dir)))
		for (const dir of chain) this.expanded[dir] = true
		if (isDir) {
			this.expanded[rel] = true
			await this.list(rel)
		}
		this.focus = { rel }
		return 'shown'
	}

	async open(rel: string): Promise<FileBuffer> {
		const existing = this.buffers[rel]
		if (existing && existing.state !== 'missing') return existing
		const root = ws.dir
		this.buffers[rel] = { rel, original: '', text: '', state: 'loading', size: 0 }
		if (!root) return this.buffers[rel]
		try {
			const file = await fs.read(root, rel)
			const state: BufferState = file.missing
				? 'missing'
				: file.tooLarge
					? 'large'
					: file.binary
						? 'binary'
						: 'text'
			const text = file.text ?? ''
			this.buffers[rel] = { rel, original: text, text, state, size: file.size }
		} catch (error) {
			toasts.error(error)
			this.buffers[rel] = { rel, original: '', text: '', state: 'missing', size: 0 }
		}
		return this.buffers[rel]
	}

	adopt(rel: string, original: string, text: string): void {
		this.buffers[rel] = { rel, original, text, state: 'text', size: text.length }
	}

	edit(rel: string, text: string): void {
		const buffer = this.buffers[rel]
		if (buffer) buffer.text = text
	}

	revert(rel: string): void {
		const buffer = this.buffers[rel]
		if (buffer) buffer.text = buffer.original
	}

	forget(rel: string): void {
		const inside = `${rel}/`
		for (const key of Object.keys(this.buffers)) {
			if (key === rel || key.startsWith(inside)) delete this.buffers[key]
		}
		for (const key of Object.keys(this.listings)) {
			if (key === rel || key.startsWith(inside)) delete this.listings[key]
		}
		for (const key of Object.keys(this.expanded)) {
			if (key === rel || key.startsWith(inside)) delete this.expanded[key]
		}
	}

	async reload(rel: string): Promise<void> {
		delete this.buffers[rel]
		await this.open(rel)
	}

	async save(rel: string): Promise<boolean> {
		const root = ws.dir
		const buffer = this.buffers[rel]
		if (!root || !buffer || buffer.state !== 'text') return false
		const text = buffer.text
		const original = buffer.original
		try {
			await fs.write(root, rel, text)
			buffer.original = text
			ws.changed()
			const inOverrides = rel.startsWith(`${OVERRIDES}/`)
			const ignored =
				inOverrides && !!instance.ignoredEntryFor(`file:${rel.slice(OVERRIDES.length + 1)}`)
			const pushable = inOverrides && instance.bound && app.prefs.pushOnSave && !ignored
			if (!pushable) {
				toasts.show(`Saved ${rel}`)
				return true
			}
			const pushed = await tasks.run(`Pushing ${rel}`, () => autoPushFile(root, rel, original))
			toasts.show(
				pushed
					? `Saved and pushed to ${instance.name}`
					: `Saved, not pushed: ${instance.name} changed it too`,
			)
			return true
		} catch (error) {
			toasts.error(error)
			return false
		}
	}

	async create(dir: string, name: string, folder: boolean): Promise<string | null> {
		const root = ws.dir
		if (!root || !name.trim()) return null
		const rel = join(dir, name.trim())
		try {
			if (folder) {
				await fs.mkdir(root, rel)
			} else {
				await fs.write(root, rel, '')
			}
			this.#index = null
			await this.list(dir)
			this.expanded[dir] = true
			ws.changed()
			return rel
		} catch (error) {
			toasts.error(error)
			return null
		}
	}

	async remove(rel: string): Promise<boolean> {
		const root = ws.dir
		if (!root) return false
		try {
			await fs.remove(root, rel)
			this.#index = null
			this.forget(rel)
			await this.list(dirname(rel))
			ws.changed()
			return true
		} catch (error) {
			toasts.error(error)
			return false
		}
	}

	async rename(rel: string, name: string): Promise<string | null> {
		const root = ws.dir
		if (!root || !name.trim()) return null
		const target = join(dirname(rel), name.trim())
		try {
			await fs.rename(root, rel, target)
			this.#index = null
			for (const key of Object.keys(this.buffers)) {
				if (key !== rel && !key.startsWith(`${rel}/`)) continue
				const moved = target + key.slice(rel.length)
				this.buffers[moved] = { ...this.buffers[key], rel: moved }
				delete this.buffers[key]
			}
			await this.list(dirname(rel))
			ws.changed()
			return target
		} catch (error) {
			toasts.error(error)
			return null
		}
	}
}

export const files = new FilesState()

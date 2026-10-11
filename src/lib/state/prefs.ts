export type ThemePref = 'system' | 'light' | 'dark'
export type KeymapPreset = 'packweave' | 'vscode' | 'jetbrains'
export type PullStrategy = 'ff' | 'merge' | 'rebase'
export type NewBranchInstance = 'copy' | 'move' | 'none'
export type DeletedBranchInstance = 'ask' | 'delete' | 'keep'
export type UpdateSource = 'github' | 'url' | 'none'

export interface RecentPack {
	dir: string
	name: string
	minecraft: string
	loader: string
	lastOpened: number
}

export interface ReleasePrefs {
	updates: UpdateSource
	url: string
	repo: string
	publishRepo: string
	modrinthProject: string
	curseforgeProject: string
	exports: {
		mrpack: boolean
		curseforge: boolean
		instance: boolean
		server: boolean
		client: boolean
	}
	publish: { github: boolean; modrinth: boolean; curseforge: boolean }
}

export interface PackPrefs {
	release: ReleasePrefs
	newBranchInstance: NewBranchInstance
	deletedBranchInstance: DeletedBranchInstance
}

export const UI_SCALES = [0.8, 0.9, 1, 1.1, 1.25, 1.5]
export const EDITOR_SIZES = [11, 12, 13, 14, 16]

export function codeLine(size: number): number {
	return Math.round(size * 1.65)
}

export interface Prefs {
	theme: ThemePref
	uiScale: number
	editorSize: number
	recents: RecentPack[]
	pushOnSave: boolean
	hiddenProblems: string[]
	keymap: { preset: KeymapPreset; custom: Record<string, string> }
	pullStrategy: PullStrategy
	gameLogOnLaunch: boolean
	packs: Record<string, PackPrefs>
}

export function defaultPrefs(): Prefs {
	return {
		theme: 'system',
		uiScale: 1,
		editorSize: 12,
		recents: [],
		pushOnSave: true,
		hiddenProblems: [],
		keymap: { preset: 'packweave', custom: {} },
		pullStrategy: 'ff',
		gameLogOnLaunch: false,
		packs: {},
	}
}

export function defaultPackPrefs(): PackPrefs {
	return {
		release: {
			updates: 'none',
			url: '',
			repo: '',
			publishRepo: '',
			modrinthProject: '',
			curseforgeProject: '',
			exports: { mrpack: true, curseforge: false, instance: false, server: false, client: false },
			publish: { github: false, modrinth: false, curseforge: false },
		},
		newBranchInstance: 'copy',
		deletedBranchInstance: 'ask',
	}
}

type Raw = Record<string, unknown>

function isRecord(value: unknown): value is Raw {
	return typeof value === 'object' && value !== null && !Array.isArray(value)
}

function pick<T extends string>(value: unknown, allowed: readonly T[], fallback: T): T {
	return allowed.includes(value as T) ? (value as T) : fallback
}

function bool(value: unknown, fallback: boolean): boolean {
	return typeof value === 'boolean' ? value : fallback
}

function text(value: unknown, fallback = ''): string {
	return typeof value === 'string' ? value : fallback
}

function flags<T extends Record<string, boolean>>(value: unknown, fallback: T): T {
	const out = { ...fallback }
	if (!isRecord(value)) return out
	for (const key of Object.keys(fallback) as (keyof T)[]) {
		out[key] = bool(value[key as string], fallback[key]) as T[keyof T]
	}
	return out
}

function readRecent(value: unknown): RecentPack | null {
	if (!isRecord(value) || typeof value.dir !== 'string') return null
	return {
		dir: value.dir,
		name: text(value.name, value.dir),
		minecraft: text(value.minecraft),
		loader: text(value.loader),
		lastOpened: typeof value.lastOpened === 'number' ? value.lastOpened : 0,
	}
}

function readPack(value: unknown): PackPrefs {
	const base = defaultPackPrefs()
	if (!isRecord(value)) return base
	const release = isRecord(value.release) ? value.release : {}
	return {
		release: {
			updates: pick(release.updates, ['github', 'url', 'none'], base.release.updates),
			url: text(release.url),
			repo: text(release.repo),
			publishRepo: text(release.publishRepo),
			modrinthProject: text(release.modrinthProject),
			curseforgeProject: text(release.curseforgeProject),
			exports: flags(release.exports, base.release.exports),
			publish: flags(release.publish, base.release.publish),
		},
		newBranchInstance: pick(
			value.newBranchInstance,
			['copy', 'move', 'none'],
			base.newBranchInstance,
		),
		deletedBranchInstance: pick(
			value.deletedBranchInstance,
			['ask', 'delete', 'keep'],
			base.deletedBranchInstance,
		),
	}
}

export function parsePrefs(json: string): Prefs {
	const base = defaultPrefs()
	const raw: unknown = JSON.parse(json)
	if (!isRecord(raw)) return base
	const keymap = isRecord(raw.keymap) ? raw.keymap : {}
	const custom: Record<string, string> = {}
	if (isRecord(keymap.custom)) {
		for (const [id, combo] of Object.entries(keymap.custom)) {
			if (typeof combo === 'string') custom[id] = combo
		}
	}
	const packs: Record<string, PackPrefs> = {}
	if (isRecord(raw.packs)) {
		for (const [dir, value] of Object.entries(raw.packs)) packs[dir] = readPack(value)
	}
	return {
		theme: pick(raw.theme, ['system', 'light', 'dark'], base.theme),
		uiScale: UI_SCALES.includes(raw.uiScale as number) ? (raw.uiScale as number) : base.uiScale,
		editorSize: EDITOR_SIZES.includes(raw.editorSize as number)
			? (raw.editorSize as number)
			: base.editorSize,
		recents: Array.isArray(raw.recents)
			? raw.recents.map(readRecent).filter((r): r is RecentPack => r !== null)
			: [],
		pushOnSave: bool(raw.pushOnSave, base.pushOnSave),
		hiddenProblems: Array.isArray(raw.hiddenProblems)
			? raw.hiddenProblems.filter((p): p is string => typeof p === 'string')
			: [],
		keymap: {
			preset: pick(keymap.preset, ['packweave', 'vscode', 'jetbrains'], base.keymap.preset),
			custom,
		},
		pullStrategy: pick(raw.pullStrategy, ['ff', 'merge', 'rebase'], base.pullStrategy),
		gameLogOnLaunch: bool(raw.gameLogOnLaunch, base.gameLogOnLaunch),
		packs,
	}
}

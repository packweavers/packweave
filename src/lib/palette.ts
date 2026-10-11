import {
	Blocks,
	File,
	GitBranch,
	GitCommitHorizontal,
	Monitor,
	Package,
	Tag,
	Terminal,
} from '@lucide/svelte'
import type { Component } from 'svelte'

import { openBranch, switchBranch } from '@/lib/flows/branches'
import { linkInstanceFolder } from '@/lib/flows/instance'
import { allCommands, isAvailable, keysFor } from '@/lib/state/commands.svelte'
import { content } from '@/lib/state/content.svelte'
import { git } from '@/lib/state/git.svelte'
import { instance } from '@/lib/state/instance.svelte'
import { formatCombo } from '@/lib/state/keymap'
import { palette } from '@/lib/state/palette.svelte'
import { tabs } from '@/lib/state/tabs.svelte'
import { tasks } from '@/lib/state/tasks.svelte'
import { ws } from '@/lib/state/workspace.svelte'
import { search as searchCatalog } from '@/lib/tauri/catalog'
import type { CatalogHit, DetectedInstance, ProviderId } from '@/lib/tauri/types'
import { providerName } from '@/lib/util/format'
import { basename } from '@/lib/util/path'
import { fuzzyScore } from '@/lib/util/text'

export type OpenHow = 'open' | 'side' | 'window'

export interface PaletteItem {
	id: string
	group: string
	label: string
	detail?: string
	hint?: string
	keys?: string | null
	icon: Component
	score: number
	run: (how: OpenHow) => void
}

export type PaletteMode =
	'all' | 'goto' | 'commands' | 'content' | 'catalog' | 'files' | 'git' | 'instances'

const PREFIXES: Record<string, PaletteMode> = {
	'>': 'commands',
	'@': 'content',
	'+': 'catalog',
	'/': 'files',
	'#': 'git',
	'!': 'instances',
}

export const GROUP_ORDER = [
	'In pack',
	'Files',
	'Commands',
	'Branches',
	'Tags',
	'Commits',
	'Instances',
	'Catalog',
]

export function parseQuery(raw: string, base: PaletteMode): { mode: PaletteMode; text: string } {
	const mode = PREFIXES[raw.charAt(0)]
	return mode ? { mode, text: raw.slice(1).trim() } : { mode: base, text: raw.trim() }
}

function ranked(items: PaletteItem[], limit: number): PaletteItem[] {
	return items.sort((a, b) => b.score - a.score).slice(0, limit)
}

function score(text: string, label: string): number | null {
	return text ? fuzzyScore(text, label) : 0
}

export function commandItems(text: string): PaletteItem[] {
	const items: PaletteItem[] = []
	for (const command of allCommands()) {
		if (!isAvailable(command)) continue
		const s = score(text, command.title)
		if (s === null) continue
		items.push({
			id: `command:${command.id}`,
			group: 'Commands',
			label: command.title,
			keys: keysFor(command.id),
			icon: command.icon ?? Terminal,
			score: s,
			run: () => void command.run(),
		})
	}
	return ranked(items, text ? 30 : 400)
}

export function contentItems(text: string): PaletteItem[] {
	const items: PaletteItem[] = []
	for (const mod of content.mods) {
		const s = score(text, mod.name)
		if (s === null) continue
		const version = mod.sources[mod.preferred]?.versionNumber ?? ''
		items.push({
			id: `mod:${mod.key}`,
			group: 'In pack',
			label: mod.name,
			detail: [version, mod.dependencyType === 'dependency' ? 'dependency' : '']
				.filter(Boolean)
				.join(' · '),
			hint: `${formatCombo('Enter')} open`,
			icon: Blocks,
			score: s,
			run: (how) => tabs.open('mod', mod.key, { side: how === 'side' }),
		})
	}
	return ranked(items, text ? 12 : 50)
}

export function fileItems(text: string, index: string[]): PaletteItem[] {
	const items: PaletteItem[] = []
	for (const rel of index) {
		const s = score(text, basename(rel)) ?? (text ? score(text, rel) : null)
		if (s === null) continue
		items.push({
			id: `file:${rel}`,
			group: 'Files',
			label: basename(rel),
			detail: rel,
			icon: File,
			score: s,
			run: (how) => tabs.open('file', rel, { side: how === 'side' }),
		})
	}
	return ranked(items, text ? 12 : 30)
}

export function gitItems(text: string): PaletteItem[] {
	const items: PaletteItem[] = []
	for (const branch of [...git.locals, ...git.remoteOnly]) {
		const s = score(text, branch.name)
		if (s === null) continue
		items.push({
			id: `branch:${branch.name}`,
			group: 'Branches',
			label: branch.name,
			detail:
				branch.name === git.branch ? 'this window' : branch.remote ? 'remote' : (branch.last ?? ''),
			hint: `${formatCombo('Enter')} switch · ${formatCombo('Alt+Enter')} new window`,
			icon: GitBranch,
			score: s,
			run: (how) => {
				if (how === 'window' || palette.preferWindow) {
					void openBranch(branch.name)
				} else {
					void switchBranch(branch.name)
				}
			},
		})
	}
	for (const tag of git.tags) {
		const s = score(text, tag.name)
		if (s === null) continue
		items.push({
			id: `tag:${tag.name}`,
			group: 'Tags',
			label: tag.name,
			detail: tag.subject,
			icon: Tag,
			score: s,
			run: () => tabs.open('history', tag.commit),
		})
	}
	return ranked(items, text ? 15 : 40)
}

export function commitItems(
	text: string,
	commits: { hash: string; short: string; subject: string }[],
): PaletteItem[] {
	const items: PaletteItem[] = []
	for (const commit of commits) {
		const s = score(text, commit.subject) ?? score(text, commit.hash)
		if (s === null) continue
		items.push({
			id: `commit:${commit.hash}`,
			group: 'Commits',
			label: commit.subject,
			detail: commit.short,
			icon: GitCommitHorizontal,
			score: s,
			run: () => tabs.open('history', commit.hash),
		})
	}
	return ranked(items, 10)
}

export function instanceItems(text: string, detected: DetectedInstance[]): PaletteItem[] {
	const items: PaletteItem[] = []
	for (const found of detected) {
		const label = `${found.name} · ${found.launcher}`
		const s = score(text, label)
		if (s === null) continue
		const current = instance.binding?.gameDir === found.gameDir
		items.push({
			id: `instance:${found.gameDir}`,
			group: 'Instances',
			label: found.name,
			detail: current ? 'linked' : found.launcher,
			hint: `${formatCombo('Enter')} ${current ? 'open' : 'link'}`,
			icon: Monitor,
			score: s,
			run: () => {
				if (current) {
					tabs.open('instance')
				} else {
					void linkInstanceFolder(found.gameDir)
				}
			},
		})
	}
	return ranked(items, 15)
}

export async function catalogItems(
	text: string,
	provider: ProviderId = 'modrinth',
): Promise<PaletteItem[]> {
	const manifest = ws.manifest
	if (!manifest || text.length < 2) return []
	const hits: CatalogHit[] = await searchCatalog({
		provider,
		query: text,
		minecraft: manifest.minecraft,
		loader: manifest.loader,
		projectType: 'mod',
		offset: 0,
		limit: 8,
	}).catch(tasks.or(`Searching ${providerName(provider)}`, [] as CatalogHit[]))
	return hits.map((hit, i) => {
		const held = content.holder(provider, hit.projectId)
		return {
			id: `catalog:${hit.projectId}`,
			group: 'Catalog',
			label: hit.title,
			detail: held ? 'in pack' : hit.author,
			hint: `${formatCombo('Enter')} ${held ? 'open' : 'add'}`,
			icon: Package,
			score: 100 - i,
			run: () => {
				if (held) {
					tabs.open('mod', held.key)
					return
				}
				void content.add([
					{
						provider,
						projectId: hit.projectId,
						slug: hit.slug,
						name: hit.title,
						projectType: 'mod',
					},
				])
			},
		}
	})
}

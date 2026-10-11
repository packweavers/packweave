import type {
	AvailableUpdate,
	ItemChange,
	LocalContent,
	LockedMod,
	ModMeta,
	ProjectType,
	ProviderId,
} from '@/lib/tauri/types'
import { providerName, sideName } from '@/lib/util/format'

import type { ContentChange } from './changes'

export type ContentFilter =
	| 'all'
	| 'updates'
	| 'uncommitted'
	| 'problems'
	| 'dependencies'
	| 'client'
	| 'server'
	| 'disabled'
	| 'pinned'
	| 'overrides'

export const FILTER_LABELS: Record<ContentFilter, string> = {
	all: 'All',
	updates: 'Updates available',
	uncommitted: 'Uncommitted',
	problems: 'Problems',
	dependencies: 'Dependencies',
	overrides: 'Overrides',
	client: 'Client only',
	server: 'Server only',
	disabled: 'Disabled',
	pinned: 'Pinned',
}

export interface ContentRow {
	id: string
	mod: LockedMod | null
	removed: ItemChange | null
	local: LocalContent | null
	icon: string | null
	author: string | null
	authorIcon: string | null
	name: string
	version: string
	update: AvailableUpdate | null
	side: string
	source: string
	otherSources: string[]
	usedBy: string[]
	change: ContentChange | null
	words: string[]
	problem: boolean
	search: string
}

export interface RowInputs {
	mods: LockedMod[]
	removed: ItemChange[]
	local: LocalContent[]
	meta: Record<string, ModMeta>
	updates: Record<string, AvailableUpdate>
	changeOf: (key: string) => ContentChange | null
	problemKeys: Set<string>
	nameOf: (id: string) => string | undefined
}

function searchText(parts: (string | null | undefined)[]): string {
	return parts
		.filter((part): part is string => !!part)
		.join('\n')
		.toLowerCase()
}

function modRow(mod: LockedMod, input: RowInputs): ContentRow {
	const source = mod.sources[mod.preferred]
	const linked = Object.keys(mod.sources) as ProviderId[]
	const files = Object.values(mod.sources)
	const words: string[] = []
	if (mod.dependencyType === 'dependency') words.push('dependency')
	if (mod.disabled) words.push('disabled')
	if (mod.noBundle) words.push('not bundled')
	const meta = input.meta[mod.projectId]
	return {
		id: mod.key,
		mod,
		removed: null,
		local: null,
		icon: meta?.iconUrl ?? null,
		author: meta?.author ?? null,
		authorIcon: meta?.authorIconUrl ?? null,
		name: mod.name,
		version: source?.versionNumber ?? '',
		update: input.updates[mod.key] ?? null,
		side: sideName(mod.clientSide, mod.serverSide),
		source: providerName(mod.preferred),
		otherSources: linked.filter((p) => p !== mod.preferred).map(providerName),
		usedBy: mod.dependents.map((id) => input.nameOf(id) ?? id),
		change: input.changeOf(mod.key),
		words,
		problem: input.problemKeys.has(mod.key),
		search: searchText([
			mod.name,
			meta?.author,
			...linked.map(providerName),
			...files.flatMap((file) => [file?.slug, file?.filename]),
		]),
	}
}

function removedRow(item: ItemChange): ContentRow {
	return {
		id: `removed:${item.path}`,
		mod: null,
		removed: item,
		local: null,
		icon: null,
		author: null,
		authorIcon: null,
		name: item.name,
		version: item.fromVersion ?? '',
		update: null,
		side: '',
		source: providerName(item.fromProvider),
		otherSources: [],
		usedBy: [],
		change: 'removed',
		words: ['removed'],
		problem: false,
		search: searchText([item.name, item.slug, providerName(item.fromProvider)]),
	}
}

function sideOf(file: LocalContent): string {
	if (file.meta?.side === 'client') return 'Client'
	return file.meta?.side === 'server' ? 'Server' : '—'
}

function localRow(file: LocalContent): ContentRow {
	const meta = file.meta
	return {
		id: `local:${file.relPath}`,
		mod: null,
		removed: null,
		local: file,
		icon: meta?.icon ?? null,
		author: meta?.authors?.join(', ') || null,
		authorIcon: null,
		name: meta?.name ?? file.filename,
		version: meta?.version ?? '',
		update: null,
		side: sideOf(file),
		source: 'Override',
		otherSources: [],
		usedBy: [],
		change: null,
		words: ['override'],
		problem: false,
		search: searchText([meta?.name, file.filename, meta?.authors?.join(' '), 'override']),
	}
}

function keep(row: ContentRow, filter: ContentFilter): boolean {
	const mod = row.mod
	const side = row.local?.meta?.side
	switch (filter) {
		case 'all':
			return true
		case 'updates':
			return row.update !== null
		case 'uncommitted':
			return row.change !== null
		case 'problems':
			return row.problem
		case 'dependencies':
			return mod?.dependencyType === 'dependency'
		case 'client':
			return side === 'client' || (!!mod && mod.serverSide === 'unsupported')
		case 'server':
			return side === 'server' || (!!mod && mod.clientSide === 'unsupported')
		case 'disabled':
			return mod?.disabled === true
		case 'pinned':
			return mod?.pinned === true
		case 'overrides':
			return row.local !== null
	}
}

export function buildRows(
	type: ProjectType,
	input: RowInputs,
	filter: ContentFilter,
	query: string,
): ContentRow[] {
	const terms = query.trim().toLowerCase().split(/\s+/).filter(Boolean)
	const rows = [
		...input.mods.filter((m) => m.projectType === type).map((m) => modRow(m, input)),
		...input.removed.filter((r) => r.projectType === type).map(removedRow),
		...input.local.filter((f) => f.projectType === type).map(localRow),
	]
	return rows
		.filter((row) => keep(row, filter))
		.filter((row) => terms.every((term) => row.search.includes(term)))
		.sort((a, b) => a.name.localeCompare(b.name, undefined, { sensitivity: 'base' }))
}

export function filterCounts(type: ProjectType, input: RowInputs): Record<ContentFilter, number> {
	const filters: ContentFilter[] = [
		'all',
		'updates',
		'uncommitted',
		'problems',
		'dependencies',
		'client',
		'server',
		'disabled',
		'pinned',
		'overrides',
	]
	return Object.fromEntries(
		filters.map((f) => [f, buildRows(type, input, f, '').length]),
	) as Record<ContentFilter, number>
}

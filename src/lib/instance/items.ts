import type {
	ChangeWord,
	EnvChange,
	FileChange,
	ModChange,
	SyncIgnored,
	SyncOp,
	SyncReport,
	SyncSide,
} from '@/lib/tauri/types'
import { gameLabel } from '@/lib/util/format'
import { basename } from '@/lib/util/path'

export type Direction = 'push' | 'pull'

export const SIDE_WORDS: Record<SyncSide, string> = {
	push: 'to push',
	pull: 'to pull',
	both: 'changed on both',
}

export interface SyncItem {
	key: string
	kind: 'mod' | 'file' | 'env'
	rawKind: string
	label: string
	detail: string
	path: string | null
	change: ChangeWord
	side: SyncSide
	modified: number | null
	ignored: boolean
	op: (direction: Direction) => SyncOp
}

function modDetail(m: ModChange): string {
	if (m.kind === 'version_diff') {
		const [from, to] =
			m.side === 'pull' ? [m.packVersion, m.instanceVersion] : [m.instanceVersion, m.packVersion]
		return `${from ?? '?'} → ${to ?? '?'}`
	}
	return m.packVersion ?? m.instanceVersion ?? ''
}

function modItem(m: ModChange): SyncItem {
	return {
		key: m.key,
		kind: 'mod',
		rawKind: m.kind,
		label: m.name,
		detail: modDetail(m),
		path: null,
		change: m.change,
		side: m.side,
		modified: m.modified,
		ignored: m.ignored,
		op: (direction) => ({
			target: 'mod',
			kind: m.kind,
			direction,
			projectId: m.projectId ?? undefined,
			slug: m.slug ?? undefined,
			provider: m.provider ?? undefined,
			name: m.name,
			instanceVersionId: m.instanceVersionId ?? undefined,
			filename: m.filename ?? undefined,
			relPath: m.relPath ?? undefined,
			projectType: m.projectType,
		}),
	}
}

function fileItem(f: FileChange): SyncItem {
	return {
		key: f.key,
		kind: 'file',
		rawKind: f.kind,
		label: basename(f.path),
		detail: '',
		path: f.path,
		change: f.change,
		side: f.side,
		modified: f.modified,
		ignored: f.ignored,
		op: (direction) => ({ target: 'file', kind: f.kind, direction, path: f.path }),
	}
}

function envItem(e: EnvChange): SyncItem {
	const pack = gameLabel(e.packMinecraft, e.packLoader, e.packLoaderVersion)
	const inst = gameLabel(
		e.instanceMinecraft ?? '?',
		e.instanceLoader ?? 'vanilla',
		e.instanceLoaderVersion,
	)
	return {
		key: e.key,
		kind: 'env',
		rawKind: 'env',
		label: 'Minecraft and loader',
		detail: e.side === 'pull' ? `${pack} → ${inst}` : `${inst} → ${pack}`,
		path: null,
		change: 'changed',
		side: e.side,
		modified: null,
		ignored: e.ignored,
		op: (direction) => ({ target: 'env', kind: 'env', direction }),
	}
}

export function syncItems(report: SyncReport | null): SyncItem[] {
	if (!report) return []
	return [
		...(report.env ? [envItem(report.env)] : []),
		...report.mods.map(modItem),
		...report.files.map(fileItem),
	]
}

export function opsFor(items: SyncItem[], directions: Direction[]): SyncOp[] {
	return items.map((item, i) => item.op(directions[i]))
}

export function coversKey(entry: SyncIgnored, key: string): boolean {
	return entry.key === key || (entry.key.endsWith('/') && key.startsWith(entry.key))
}

export function ignoreEntry(item: SyncItem): SyncIgnored {
	return { key: item.key, label: item.path ?? item.label }
}

export function folderEntry(path: string): SyncIgnored {
	return { key: `file:${path}/`, label: `${path}/` }
}

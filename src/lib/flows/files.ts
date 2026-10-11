import ConfirmDialog from '@/dialogs/ConfirmDialog.svelte'
import { content } from '@/lib/state/content.svelte'
import { dialogs } from '@/lib/state/dialogs.svelte'
import { files } from '@/lib/state/files.svelte'
import { layout } from '@/lib/state/layout.svelte'
import { tabs } from '@/lib/state/tabs.svelte'
import { toasts } from '@/lib/state/toasts.svelte'
import { ws } from '@/lib/state/workspace.svelte'
import { defaultIgnore, readIgnore, writeIgnore } from '@/lib/tauri/instance'
import type { IgnoreKind } from '@/lib/tauri/types'
import { reveal } from '@/lib/tauri/windows'
import { basename, join } from '@/lib/util/path'

export async function deleteFile(rel: string): Promise<boolean> {
	if (!(await files.remove(rel))) return false
	void content.refreshHistory()
	toasts.show(`Deleted ${basename(rel)}`, [
		{ label: 'Undo', command: 'edit.undo', run: () => void content.undo() },
	])
	return true
}

export const IGNORE_FILES: Record<IgnoreKind, { file: string; hint: string }> = {
	git: {
		file: '.gitignore',
		hint: 'Ignored by Git and Instance Sync',
	},
	export: {
		file: '.packweaveignore',
		hint: 'Not included in Exports',
	},
}

export function ignoreKindOf(rel: string): IgnoreKind | null {
	if (rel === IGNORE_FILES.git.file) return 'git'
	return rel === IGNORE_FILES.export.file ? 'export' : null
}

export async function openIgnoreFile(kind: IgnoreKind): Promise<void> {
	const dir = ws.dir
	if (!dir) return
	const { file, hint } = IGNORE_FILES[kind]
	const entries = await files.list('')
	if (!entries.some((e) => e.name === file)) {
		const ok = await dialogs.open(ConfirmDialog, {
			title: `Create ${file}?`,
			message: hint,
			confirm: 'Create',
		})
		if (!ok) return
		try {
			const existing = await readIgnore(dir, kind)
			await writeIgnore(dir, kind, existing || (await defaultIgnore(kind)))
			await files.list('')
			ws.changed()
		} catch (error) {
			toasts.error(error)
			return
		}
	}
	tabs.open('file', file)
}

export function openFile(rel: string, preview = false): void {
	const kind = ignoreKindOf(rel)
	if (kind) {
		void openIgnoreFile(kind)
		return
	}
	tabs.open('file', rel, { preview })
}

export async function showInFiles(rel: string, isDir: boolean): Promise<boolean> {
	const dir = ws.dir
	if (!dir) return false
	const spot = await files.locate(rel, isDir)
	if (spot === 'missing') return false
	if (spot === 'shown') layout.showView('files')
	if (!isDir) {
		openFile(rel)
	} else if (spot === 'found') {
		await reveal(join(dir, rel))
	}
	return true
}

export async function openInFiles(rel: string, isDir: boolean): Promise<void> {
	if (!(await showInFiles(rel, isDir))) toasts.info(`${basename(rel)} isn't in the pack`)
}

export async function openSetting(rel: string, key: string): Promise<void> {
	files.reveal = { rel, line: 0, key }
	if (!(await showInFiles(rel, false))) toasts.info(`${basename(rel)} isn't in the pack`)
}

export async function resetIgnoreFile(kind: IgnoreKind): Promise<boolean> {
	const dir = ws.dir
	if (!dir) return false
	const { file } = IGNORE_FILES[kind]
	const ok = await dialogs.open(ConfirmDialog, {
		title: `Reset ${file} to defaults?`,
		message: 'Lines you added are removed.',
		confirm: 'Reset',
		danger: true,
	})
	if (!ok) return false
	try {
		await writeIgnore(dir, kind, await defaultIgnore(kind))
		files.forget(file)
		await files.reload(file)
		ws.changed()
		toasts.show(`Reset ${file}`)
		return true
	} catch (error) {
		toasts.error(error)
		return false
	}
}

export async function createMissingFile(rel: string): Promise<void> {
	const kind = ignoreKindOf(rel)
	if (kind) {
		await openIgnoreFile(kind)
		await files.reload(rel)
		return
	}
	const slash = rel.lastIndexOf('/')
	const created = await files.create(
		slash < 0 ? '' : rel.slice(0, slash),
		rel.slice(slash + 1),
		false,
	)
	if (created) await files.reload(rel)
}

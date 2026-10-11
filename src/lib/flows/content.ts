import DropFilesDialog from '@/dialogs/DropFilesDialog.svelte'
import FindSourcesDialog from '@/dialogs/FindSourcesDialog.svelte'
import PasteListDialog from '@/dialogs/PasteListDialog.svelte'
import RemoveDialog from '@/dialogs/RemoveDialog.svelte'
import UpdateDialog from '@/dialogs/UpdateDialog.svelte'
import { content } from '@/lib/state/content.svelte'
import { dialogs } from '@/lib/state/dialogs.svelte'
import { git } from '@/lib/state/git.svelte'
import { tabs } from '@/lib/state/tabs.svelte'
import { tasks } from '@/lib/state/tasks.svelte'
import { toasts } from '@/lib/state/toasts.svelte'
import { ws } from '@/lib/state/workspace.svelte'
import { errorText } from '@/lib/tauri/call'
import { convertSearch } from '@/lib/tauri/catalog'
import { addDropped, identifyDropped, previewChange } from '@/lib/tauri/content'
import type {
	ConvertCandidate,
	LocalContent,
	LockedMod,
	ProjectType,
	ProviderId,
	VersionPin,
} from '@/lib/tauri/types'
import { plural, providerName } from '@/lib/util/format'

import { deleteFile } from './files'
import { chooseDatapackFolder } from './pack'

export function openAddContent(type: ProjectType = 'mod'): void {
	tabs.open('add', type)
}

export async function updateContent(keys: string[]): Promise<void> {
	const pins = content.pinsFor(keys)
	if (pins.length === 0) {
		toasts.info('Already up to date')
		return
	}
	await updatePins(pins)
}

export function updateAll(): Promise<void> {
	return updateContent(Object.keys(content.updates))
}

export async function updatePins(pins: VersionPin[]): Promise<void> {
	const dir = ws.dir
	if (!dir) return
	const impact = await tasks
		.run('Checking what else changes', () => previewChange(dir, { versions: pins }))
		.catch(() => null)
	const asked = new Set(pins.map((p) => p.key))
	const extra = impact?.changes.filter((c) => !asked.has(c.key)) ?? []
	const quiet = impact && extra.length === 0 && impact.problems.length === 0 && pins.length === 1
	if (quiet) {
		await content.setVersions(pins)
		return
	}
	const choice = await dialogs.open(UpdateDialog, { pins, impact })
	if (choice === 'update') await content.setVersions(pins)
}

export async function removeContent(keys: string[]): Promise<void> {
	const dir = ws.dir
	if (!dir || keys.length === 0) return
	const impact = await previewChange(dir, { remove: keys }).catch(
		tasks.or('Checking what else the removal changes', null),
	)
	const removedHere = new Set(keys)
	const extra = impact?.changes.filter((c) => !removedHere.has(c.key)) ?? []
	if (extra.length === 0) {
		await content.remove(keys)
		return
	}
	const choice = await dialogs.open(RemoveDialog, { keys, impact })
	if (choice === 'remove') {
		await content.remove(keys)
	} else if (choice === 'disable') {
		await content.setDisabled(keys, true)
	}
}

export async function findSource(key: string, target: ProviderId): Promise<void> {
	await findSources([key], target)
}

export interface SourceMatch {
	key: string
	match: ConvertCandidate | null
}

async function searchAll(
	dir: string,
	mods: LockedMod[],
	target: ProviderId,
): Promise<SourceMatch[]> {
	const found = new Map<string, ConvertCandidate | null>()
	const failed: unknown[] = []
	const queue = [...mods]
	const worker = async (): Promise<void> => {
		for (let mod = queue.shift(); mod; mod = queue.shift()) {
			const match = await convertSearch(dir, mod.key, target).catch((error) => {
				failed.push(error)
				return null
			})
			found.set(mod.key, match)
		}
	}
	await Promise.all([worker(), worker(), worker(), worker()])
	if (failed.length > 0) {
		toasts.show(
			`Couldn't search ${providerName(target)} for ${plural(failed.length, 'mod', 'mods')}: ${errorText(failed[0])}`,
			[],
			'error',
		)
	}
	return mods.map((mod) => ({ key: mod.key, match: found.get(mod.key) ?? null }))
}

export async function findSources(keys: string[], target: ProviderId): Promise<void> {
	const dir = ws.dir
	if (!dir) return
	const mods = keys
		.map((key) => content.find(key))
		.filter((mod): mod is LockedMod => !!mod && !mod.sources[target])
	if (mods.length === 0) {
		toasts.info(`Already on ${providerName(target)}`)
		return
	}
	const matches = await tasks.run(
		`Searching ${providerName(target)} for ${plural(mods.length, 'mod', 'mods')}`,
		() => searchAll(dir, mods, target),
	)
	const picks = await dialogs.open(FindSourcesDialog, { target, matches })
	if (picks && picks.length > 0) await content.addSources(picks)
}

export async function toggleDisabled(keys: string[]): Promise<void> {
	const mods = keys.map((k) => content.find(k)).filter((m) => !!m)
	const disable = mods.some((m) => !m.disabled)
	await content.setDisabled(keys, disable)
}

export async function pasteList(): Promise<void> {
	const result = await dialogs.open(PasteListDialog, {})
	if (!result || result.items.length === 0) return
	if (result.datapacks && !(await chooseDatapackFolder(result.datapacks))) return
	await content.add(result.items)
}

export async function dropFiles(paths: string[]): Promise<void> {
	const dir = ws.dir
	if (!dir || paths.length === 0) return
	const scan = await tasks
		.run('Reading dropped files', () => identifyDropped(dir, paths))
		.catch((error) => {
			toasts.error(error)
			return null
		})
	if (!scan) return
	for (const failure of scan.failures) toasts.error(failure)
	if (scan.files.length === 0) return
	const chosen = await dialogs.open(DropFilesDialog, { files: scan.files })
	if (!chosen || chosen.length === 0) return
	try {
		const resolved = await tasks.run('Adding files', () => addDropped(dir, chosen))
		content.apply(resolved)
		ws.changed()
		void content.refreshHistory()
		void content.refreshUnpublished()
		toasts.show(`Added ${plural(chosen.length, 'file', 'files')}`, [
			{ label: 'Undo', command: 'edit.undo', run: () => void content.undo() },
		])
	} catch (error) {
		toasts.error(error)
	}
}

export async function restoreRemoved(path: string): Promise<void> {
	try {
		await git.discard([path])
		toasts.show('Restored')
	} catch (error) {
		toasts.error(error)
	}
}

export async function deleteLocalFile(file: LocalContent): Promise<void> {
	if (await deleteFile(`overrides/${file.relPath}`)) await content.refreshUnpublished()
}

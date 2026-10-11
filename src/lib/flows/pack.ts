import { getCurrentWindow } from '@tauri-apps/api/window'

import CloneDialog from '@/dialogs/CloneDialog.svelte'
import ConfirmDialog from '@/dialogs/ConfirmDialog.svelte'
import FromInstanceDialog from '@/dialogs/FromInstanceDialog.svelte'
import ImportDialog from '@/dialogs/ImportDialog.svelte'
import NewPackDialog from '@/dialogs/NewPackDialog.svelte'
import { app } from '@/lib/state/app.svelte'
import { dialogs } from '@/lib/state/dialogs.svelte'
import { files } from '@/lib/state/files.svelte'
import { tasks } from '@/lib/state/tasks.svelte'
import { toasts } from '@/lib/state/toasts.svelte'
import { ws } from '@/lib/state/workspace.svelte'
import { init as gitInit } from '@/lib/tauri/git'
import { bindInstance } from '@/lib/tauri/instance'
import {
	clonePack as clone,
	convertPackwiz,
	createPack,
	importPack as importFrom,
	packwizInfo,
	readManifest,
	setDatapackFolder,
} from '@/lib/tauri/pack'
import type { Converted, PackState, PackwizInfo } from '@/lib/tauri/types'
import { openPackWindow, pickFolder } from '@/lib/tauri/windows'
import { plural } from '@/lib/util/format'
import { basename, join } from '@/lib/util/path'
import { slugify } from '@/lib/util/text'

import { chooseInstance, createInstanceFor, syncEverything } from './instance'

function reportConversion(name: string, converted: Converted | undefined): void {
	if (!converted) return
	const parts = [
		plural(converted.content, 'content item', 'content items'),
		plural(converted.files, 'file', 'files'),
	]
	if (converted.unlocked > 0) parts.push(`${converted.unlocked} unlocked for player edits`)
	const notes = [`Converted ${name} from packwiz: ${parts.join(', ')}.`]
	if (converted.disabled.length > 0) {
		notes.push(`Optional and off by default, so disabled: ${converted.disabled.join(', ')}.`)
	}
	if (converted.skipped.length > 0) {
		notes.push(`Skipped ${plural(converted.skipped.length, 'entry', 'entries')}, see Output.`)
		for (const line of converted.skipped) tasks.note(`packwiz: ${line}`, 'error')
	}
	toasts.show(notes.join(' '))
}

async function convertIfPackwiz(dir: string): Promise<boolean> {
	let wiz: PackwizInfo | null
	try {
		wiz = await packwizInfo(dir)
	} catch (error) {
		toasts.error(error)
		return false
	}
	if (!wiz) return true
	const ok = await dialogs.open(ConfirmDialog, {
		title: `Convert ${wiz.name} from packwiz?`,
		message: wiz.git
			? `packweave writes modpack.json and content/ for ${plural(wiz.content, 'item', 'items')}, moves ${plural(wiz.files, 'file', 'files')} into overrides/ and removes pack.toml, index.toml and the .pw.toml files. Git keeps the packwiz version in its history.`
			: `packweave adds modpack.json, content/ and overrides/ for ${plural(wiz.content, 'item', 'items')} and ${plural(wiz.files, 'file', 'files')}. The packwiz files stay as they are.`,
		confirm: 'Convert',
	})
	if (!ok) return false
	try {
		const state = await tasks.run(`Converting ${wiz.name}`, () => convertPackwiz(dir))
		reportConversion(state.manifest.name, state.converted)
		return true
	} catch (error) {
		toasts.error(error)
		return false
	}
}

export async function openPackAt(dir: string, newWindow = false): Promise<void> {
	if (!(await convertIfPackwiz(dir))) return
	if (ws.isOpen || newWindow) {
		const manifest = await readManifest(dir).catch((error) => {
			toasts.error(error)
			return null
		})
		if (manifest) await openPackWindow(dir, manifest.name)
		return
	}
	if ((await ws.open(dir)) === 'failed') app.removeRecent(dir)
}

export async function openPackFolder(): Promise<void> {
	const dir = await pickFolder('Open a pack folder')
	if (dir) await openPackAt(dir)
}

async function startGit(dir: string, wanted: boolean): Promise<void> {
	if (!wanted) return
	await gitInit(dir).catch((error) => toasts.error(error))
}

export async function newPack(): Promise<void> {
	const result = await dialogs.open(NewPackDialog, {})
	if (!result) return
	const dir = join(result.parent, slugify(result.name))
	try {
		await tasks.run(`Creating ${result.name}`, () =>
			createPack(dir, result.name, result.minecraft, result.loader, result.loaderVersion),
		)
		await startGit(dir, result.git)
		if (result.prism) await createInstanceFor(dir).catch((error) => toasts.error(error))
		await openPackAt(dir)
	} catch (error) {
		toasts.error(error)
	}
}

export async function importPack(source: string | null = null): Promise<void> {
	const result = await dialogs.open(ImportDialog, { source })
	if (!result) return
	try {
		const state = await tasks.run('Importing', () => importFrom(result.source, result.dest))
		await startGit(state.dir, result.git)
		reportConversion(state.manifest.name, state.converted)
		await openPackAt(state.dir)
	} catch (error) {
		toasts.error(error)
	}
}

export async function importDropped(paths: string[]): Promise<void> {
	const file = paths.find((p) => /\.(mrpack|zip)$/i.test(p) || basename(p) === 'pack.toml')
	if (file) return importPack(file)
	for (const path of paths) {
		const wiz = await packwizInfo(path).catch((error) => {
			toasts.error(error)
			return null
		})
		if (wiz) return importPack(join(path, 'pack.toml'))
	}
}

async function linkCloned(state: PackState, then: 'link' | 'create' | 'skip'): Promise<void> {
	if (then === 'create') {
		await createInstanceFor(state.dir)
	} else if (then === 'link') {
		await chooseInstance(state.dir, state.manifest, state.manifest.name, null)
	}
}

export async function clonePack(): Promise<void> {
	const result = await dialogs.open(CloneDialog, {})
	if (!result) return
	try {
		const state = await tasks.run('Cloning', () => clone(result.url, result.parent))
		reportConversion(state.manifest.name, state.converted)
		await linkCloned(state, result.then).catch((error) => toasts.error(error))
		await openPackAt(state.dir)
	} catch (error) {
		toasts.error(error)
	}
}

export async function packFromInstance(): Promise<void> {
	const result = await dialogs.open(FromInstanceDialog, {})
	if (!result) return
	const found = result.instance
	const name = found.packName ?? found.name
	const dir = join(result.parent, slugify(name))
	try {
		await tasks.run(`Creating ${name}`, () =>
			createPack(
				dir,
				name,
				found.minecraft ?? '1.21.1',
				found.loader ?? 'vanilla',
				found.loaderVersion,
			),
		)
		await startGit(dir, result.git)
		await bindInstance(dir, found.gameDir)
		await syncEverything(dir, 'pull', `Pulling from ${found.name}`).catch((error) =>
			toasts.error(error),
		)
		await openPackAt(dir)
	} catch (error) {
		toasts.error(error)
	}
}

export async function confirmUnsaved(): Promise<boolean> {
	const count = files.dirty.length
	if (count === 0) return true
	const ok = await dialogs.open(ConfirmDialog, {
		title: `Close with ${plural(count, 'unsaved file', 'unsaved files')}?`,
		message: 'Unsaved edits are lost.',
		confirm: 'Close without saving',
		danger: true,
	})
	return ok !== undefined
}

export async function closePack(): Promise<void> {
	if (!(await confirmUnsaved())) return
	if (ws.detached) {
		await getCurrentWindow().destroy()
		return
	}
	await ws.close()
}

export async function chooseDatapackFolder(folder: string): Promise<boolean> {
	const dir = ws.dir
	if (!dir) return false
	try {
		ws.manifest = await setDatapackFolder(dir, folder)
		ws.changed()
		toasts.show(`Data packs go in ${ws.manifest.datapacks}`)
		return true
	} catch (error) {
		toasts.error(error)
		return false
	}
}

import LinkInstanceDialog from '@/dialogs/LinkInstanceDialog.svelte'
import { syncItems, type Direction } from '@/lib/instance/items'
import { content } from '@/lib/state/content.svelte'
import { dialogs } from '@/lib/state/dialogs.svelte'
import { git } from '@/lib/state/git.svelte'
import { instance } from '@/lib/state/instance.svelte'
import { tasks } from '@/lib/state/tasks.svelte'
import { toasts } from '@/lib/state/toasts.svelte'
import { ws } from '@/lib/state/workspace.svelte'
import {
	applySync,
	bindInstance,
	canCopyInstance,
	copyInstance,
	createPrismInstance,
	syncStatus,
	unbindInstance,
} from '@/lib/tauri/instance'
import type { DetectedInstance, Manifest, Worktree } from '@/lib/tauri/types'
import { openFolder } from '@/lib/tauri/windows'
import { sameDir } from '@/lib/util/path'

export async function syncEverything(
	dir: string,
	direction: Direction,
	label: string,
): Promise<void> {
	const items = syncItems(await syncStatus(dir)).filter((i) => !i.ignored && i.side === direction)
	if (items.length === 0) return
	const ops = items.map((item) => item.op(direction))
	await tasks.run(label, () => applySync(dir, ops, label))
}

export async function createInstanceFor(dir: string): Promise<DetectedInstance> {
	const created = await tasks.run('Creating a Prism instance', () => createPrismInstance(dir))
	const linked = await bindInstance(dir, created.gameDir)
	await syncEverything(dir, 'push', `Pushing to ${linked.name}`)
	return linked
}

export async function chooseInstance(
	dir: string,
	manifest: Manifest,
	target: string,
	main: Worktree | null,
): Promise<DetectedInstance | null> {
	const mainInstance = main && !sameDir(main.path, dir) ? main.instance : null
	const choice = await dialogs.open(LinkInstanceDialog, {
		target,
		manifest,
		mainInstance,
		canCopy: mainInstance
			? await canCopyInstance(mainInstance).catch(
					tasks.or('Checking whether the instance can be copied', false),
				)
			: false,
	})
	if (!choice) return null
	switch (choice.action) {
		case 'copy': {
			const copy = await tasks.run('Copying the instance', () =>
				copyInstance(choice.source, choice.name),
			)
			return bindInstance(dir, copy.gameDir)
		}
		case 'move':
			if (main) await unbindInstance(main.path)
			return bindInstance(dir, choice.source)
		case 'create':
			return createInstanceFor(dir)
		case 'use':
			return bindInstance(dir, choice.gameDir)
	}
}

async function linked(dir: string, found: DetectedInstance): Promise<void> {
	if (ws.dir && sameDir(ws.dir, dir)) {
		await instance.load()
		void content.refreshHistory()
	}
	toasts.show(`Linked ${found.name}`)
}

export async function linkInstance(): Promise<void> {
	const dir = ws.dir
	const manifest = ws.manifest
	if (!dir || !manifest) return
	await git.refreshRefs()
	const main = git.worktrees.find((w) => w.main) ?? null
	try {
		const found = await chooseInstance(dir, manifest, git.branch ?? manifest.name, main)
		if (found) await linked(dir, found)
	} catch (error) {
		toasts.error(error)
	}
}

export async function linkInstanceFolder(gameDir: string): Promise<void> {
	const dir = ws.dir
	if (!dir) return
	try {
		await linked(dir, await bindInstance(dir, gameDir))
	} catch (error) {
		toasts.error(error)
	}
}

export async function unlinkInstance(): Promise<void> {
	const name = instance.name
	try {
		await instance.unlink()
		toasts.show(`Unlinked ${name}`)
	} catch (error) {
		toasts.error(error)
	}
}

export async function revealInstance(): Promise<void> {
	if (instance.binding) {
		await openFolder(instance.binding.gameDir)
	}
}

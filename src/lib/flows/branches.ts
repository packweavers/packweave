import ConfirmDialog from '@/dialogs/ConfirmDialog.svelte'
import DeleteBranchDialog from '@/dialogs/DeleteBranchDialog.svelte'
import MergeDialog from '@/dialogs/MergeDialog.svelte'
import NewBranchDialog from '@/dialogs/NewBranchDialog.svelte'
import PromptDialog from '@/dialogs/PromptDialog.svelte'
import SwitchBranchDialog from '@/dialogs/SwitchBranchDialog.svelte'
import TestBranchDialog from '@/dialogs/TestBranchDialog.svelte'
import { app } from '@/lib/state/app.svelte'
import { dialogs } from '@/lib/state/dialogs.svelte'
import { git } from '@/lib/state/git.svelte'
import { instance } from '@/lib/state/instance.svelte'
import { layout } from '@/lib/state/layout.svelte'
import { tabs } from '@/lib/state/tabs.svelte'
import { tasks } from '@/lib/state/tasks.svelte'
import { toasts } from '@/lib/state/toasts.svelte'
import { ws } from '@/lib/state/workspace.svelte'
import { addContent, resolvePack, setVersions } from '@/lib/tauri/content'
import * as api from '@/lib/tauri/git'
import {
	bindInstance,
	canCopyInstance,
	copyInstance,
	deleteInstance,
	unbindInstance,
} from '@/lib/tauri/instance'
import { readManifest, saveManifest } from '@/lib/tauri/pack'
import type { AddItem, Manifest, VersionPin } from '@/lib/tauri/types'
import { closePackWindow, openPackWindow } from '@/lib/tauri/windows'
import { plural } from '@/lib/util/format'
import { sameDir } from '@/lib/util/path'

import { push, withGit } from './git'

export interface BranchChange {
	label: string
	items?: AddItem[]
	pins?: VersionPin[]
	game?: Partial<Manifest>
}

function mainDir(): string | null {
	return ws.mainDir ?? ws.dir
}

function mainInstance(): string | null {
	return git.worktrees.find((w) => w.main)?.instance ?? null
}

export async function openBranch(name: string): Promise<void> {
	const main = mainDir()
	if (!main) return
	const folder = await withGit(() =>
		tasks.run(`Opening ${name}`, () => api.branchFolder(main, name, null)),
	)
	if (!folder) return
	if (ws.dir && sameDir(folder, ws.dir)) return
	const branch = git.isRemote(name) ? name.slice(name.indexOf('/') + 1) : name
	await openPackWindow(folder, `${ws.name} — ${branch}`)
	void git.refreshRefs()
}

async function applyInstanceChoice(
	folder: string,
	branch: string,
	choice: 'copy' | 'move' | 'none',
): Promise<void> {
	const source = mainInstance()
	if (!source || choice === 'none') return
	if (choice === 'move') {
		const main = mainDir()
		if (main) await unbindInstance(main)
		await bindInstance(folder, source)
		if (main && ws.dir && sameDir(main, ws.dir)) instance.reset()
		return
	}
	const copy = await tasks.run(`Copying ${instance.name || 'the instance'}`, () =>
		copyInstance(source, `${ws.name} ${branch}`),
	)
	await bindInstance(folder, copy.gameDir)
}

export async function newBranch(
	start: string | null = null,
	change: BranchChange | null = null,
): Promise<void> {
	const main = mainDir()
	if (!main) return
	if (!git.isRepo) {
		toasts.info('Start using Git for this pack first.')
		return
	}
	await git.refreshRefs()
	const source = mainInstance()
	const result = await dialogs.open(NewBranchDialog, {
		start: start ?? git.branch ?? 'main',
		branches: git.locals.map((b) => b.name),
		remotes: (git.branches?.list ?? []).filter((b) => b.remote).map((b) => b.name),
		instanceName: source ? instance.name || 'the main instance' : null,
		canCopy: source
			? await canCopyInstance(source).catch(
					tasks.or('Checking whether the instance can be copied', false),
				)
			: false,
		defaultInstance: app.pack(main).newBranchInstance,
		suggestion: change ? change.label : '',
		changeLabel: change?.label ?? null,
	})
	if (!result) return
	if (result.where === 'here' && !change) {
		const dir = ws.dir
		if (!dir) return
		const done = await withGit(async () => {
			await api.createBranch(dir, result.name, result.start, true)
			return true
		})
		if (done) await ws.reload()
		return
	}
	const folder = await withGit(() =>
		tasks.run(`Creating ${result.name}`, () => api.branchFolder(main, result.name, result.start)),
	)
	if (!folder) return
	try {
		await applyInstanceChoice(folder, result.name, result.instance)
		if (change?.items?.length) {
			await tasks.run(change.label, () => addContent(folder, change.items!))
		}
		if (change?.pins?.length) await tasks.run(change.label, () => setVersions(folder, change.pins!))
		if (change?.game) {
			const current = await readManifest(folder)
			await saveManifest(folder, { ...current, ...change.game })
			await tasks.run(change.label, () => resolvePack(folder))
		}
	} catch (error) {
		toasts.error(error)
	}
	await openPackWindow(folder, `${ws.name} — ${result.name}`)
	void git.refreshRefs()
}

export async function newTestBranch(): Promise<void> {
	if (!git.isRepo) {
		toasts.info('Start using Git for this pack first.')
		return
	}
	const change = await dialogs.open(TestBranchDialog, {})
	if (change) await newBranch(git.branch, change)
}

export async function switchBranch(name: string): Promise<void> {
	const dir = ws.dir
	if (!dir || name === git.branch) return
	const elsewhere = git.worktreeFor(name)
	if (elsewhere && !sameDir(elsewhere.path, dir)) {
		await openBranch(name)
		return
	}
	const changes = git.changes.length
	if (changes > 0) {
		const choice = await dialogs.open(SwitchBranchDialog, { branch: name, changes })
		if (!choice) return
		if (choice === 'commit') {
			layout.showView('git')
			return
		}
		if (choice === 'stash') {
			const stashed = await withGit(async () => {
				await api.stashPush(dir, `Before switching to ${name}`)
				return true
			})
			if (!stashed) return
		}
	}
	const done = await withGit(async () => {
		await tasks.run(`Switching to ${name}`, () => api.checkout(dir, name))
		return true
	})
	if (done) await ws.reload()
}

export async function renameBranch(name: string): Promise<void> {
	const dir = ws.dir
	if (!dir) return
	const next = await dialogs.open(PromptDialog, {
		title: `Rename ${name}`,
		label: 'Name',
		value: name,
		confirm: 'Rename',
	})
	if (!next || next.trim() === name) return
	await withGit(async () => {
		await api.renameBranch(dir, name, next.trim())
		await git.refreshAll()
	})
}

export async function deleteBranch(name: string): Promise<void> {
	const dir = ws.dir
	if (!dir) return
	if (name === git.branch) {
		toasts.info('Switch this window to another branch first.')
		return
	}
	const base = git.branch ?? 'HEAD'
	const compare = await api.compare(dir, base, name).catch((error) => {
		toasts.error(error)
		return null
	})
	const tree = git.worktreeFor(name)
	const result = await dialogs.open(DeleteBranchDialog, {
		branch: name,
		base,
		unmerged: compare?.commits.length ?? 0,
		hasFolder: !!tree && !tree.main,
		instanceDir: tree?.instance ?? null,
		canDeleteInstance: tree?.instance
			? await canCopyInstance(tree.instance).catch(
					tasks.or('Checking whether the instance can be copied', false),
				)
			: false,
		policy: app.pack(ws.mainDir ?? dir).deletedBranchInstance,
	})
	if (!result) return
	if (tree && !tree.main) await closePackWindow(tree.path)
	const done = await withGit(async () => {
		await tasks.run(`Deleting ${name}`, () =>
			api.deleteBranch(dir, name, (compare?.commits.length ?? 0) > 0),
		)
		return true
	})
	if (!done) return
	if (result.deleteInstance && tree?.instance) {
		await deleteInstance(tree.instance).catch((error) => toasts.error(error))
	}
	await git.refreshAll()
	toasts.show(`Deleted ${name}`)
}

export async function deleteRemoteBranch(name: string): Promise<void> {
	const dir = ws.dir
	if (!dir) return
	const ok = await dialogs.open(ConfirmDialog, {
		title: `Delete ${name}?`,
		message: 'Deletes the branch on the remote for everyone.',
		confirm: 'Delete branch',
		danger: true,
	})
	if (!ok) return
	const done = await withGit(async () => {
		await tasks.run(`Deleting ${name}`, () => api.deleteRemoteBranch(dir, name))
		return true
	})
	if (!done) return
	await git.refreshRefs()
	toasts.show(`Deleted ${name}`)
}

export async function mergeBranch(from: string): Promise<void> {
	const dir = ws.dir
	if (!dir || !git.branch) return
	const into = git.branch
	if (git.changes.length > 0) {
		toasts.info(`Commit or stash the changes on ${into} first.`)
		return
	}
	const compare = await api.compare(dir, into, from).catch((error) => {
		toasts.error(error)
		return null
	})
	const tree = git.worktreeFor(from)
	const result = await dialogs.open(MergeDialog, {
		from,
		into,
		commits: compare?.commits.length ?? 0,
		conflicts: compare?.conflicts ?? null,
		hasFolder: !!tree && !tree.main,
		instanceDir: tree?.instance ?? null,
		hasRemote: git.remoteName !== null,
		remote: git.isRemote(from),
		policy: app.pack(ws.mainDir ?? dir).deletedBranchInstance,
	})
	if (!result) return
	const conflicted = await withGit(() =>
		tasks.run(`Merging ${from} into ${into}`, () =>
			api.merge(
				dir,
				from,
				result.mode,
				`${result.mode === 'squash' ? 'Squash' : 'Merge'} ${from} into ${into}`,
			),
		),
	)
	if (conflicted === undefined) return
	await ws.reload()
	if (conflicted) {
		tabs.open('conflicts')
		toasts.info(
			`Merging ${from} stopped on ${plural(git.conflicts.length, 'conflict', 'conflicts')}`,
		)
		return
	}
	toasts.show(`Merged ${from} into ${into}`)
	if (result.push) await push()
	if (tree && !tree.main && result.instance === 'link' && tree.instance) {
		await bindInstance(dir, tree.instance)
		await unbindInstance(tree.path)
		await instance.load()
	}
	if (result.deleteBranch) {
		if (tree && !tree.main) await closePackWindow(tree.path)
		await withGit(() => api.deleteBranch(dir, from, true))
		if (result.instance === 'delete' && tree?.instance) {
			await deleteInstance(tree.instance).catch((error) => toasts.error(error))
		}
		await git.refreshAll()
	}
}

export async function abortMerge(): Promise<void> {
	const dir = ws.dir
	if (!dir) return
	const ok = await dialogs.open(ConfirmDialog, {
		title: 'Abort the merge?',
		message: 'Restores everything to before the merge.',
		confirm: 'Abort merge',
		danger: true,
	})
	if (!ok) return
	await withGit(async () => {
		await api.mergeAbort(dir)
		tabs.closeWhere((d) => d.kind === 'conflicts')
		await ws.reload()
	})
}

export async function commitMerge(): Promise<void> {
	const dir = ws.dir
	if (!dir) return
	const done = await withGit(async () => {
		await tasks.run('Committing the merge', () =>
			api.mergeCommit(dir, git.message.trim() || (git.merge?.note?.message ?? '')),
		)
		return true
	})
	if (!done) return
	git.message = ''
	tabs.closeWhere((d) => d.kind === 'conflicts')
	await ws.reload()
	toasts.show('Merge committed')
}

import ConfirmDialog from '@/dialogs/ConfirmDialog.svelte'
import GitAccessDialog from '@/dialogs/GitAccessDialog.svelte'
import GitSetupDialog from '@/dialogs/GitSetupDialog.svelte'
import PromptDialog from '@/dialogs/PromptDialog.svelte'
import { content } from '@/lib/state/content.svelte'
import { dialogs } from '@/lib/state/dialogs.svelte'
import { git } from '@/lib/state/git.svelte'
import { panels } from '@/lib/state/panels.svelte'
import { toasts } from '@/lib/state/toasts.svelte'
import { ws } from '@/lib/state/workspace.svelte'
import { errorCode } from '@/lib/tauri/call'
import * as api from '@/lib/tauri/git'
import type { ItemChange } from '@/lib/tauri/types'
import { plural } from '@/lib/util/format'

function remoteHost(): string | null {
	const url = git.remotes.find((r) => r.name === git.remoteName)?.url ?? ''
	const match = /^(?:https?:\/\/|git@)([^/:]+)/.exec(url)
	return match ? match[1].toLowerCase() : null
}

export async function withGit<T>(work: () => Promise<T>, retried = false): Promise<T | undefined> {
	try {
		return await work()
	} catch (error) {
		const code = errorCode(error)
		if (code === 'git-missing' || code === 'git-identity') {
			const done = await dialogs.open(GitSetupDialog, {})
			return done && !retried ? withGit(work, true) : undefined
		}
		if (code === 'git-auth' && !retried) {
			const saved = await dialogs.open(GitAccessDialog, { host: remoteHost() })
			return saved ? withGit(work, true) : undefined
		}
		toasts.error(error, [{ label: 'Show', run: () => panels.open('output') }])
		return undefined
	}
}

export async function initRepo(): Promise<void> {
	await withGit(() => git.init())
}

export async function commit(message: string, files: string[], amend: boolean): Promise<boolean> {
	const done = await withGit(async () => {
		await git.commit(message, files, amend)
		return true
	})
	if (done) toasts.show(`Committed "${message.split('\n')[0]}"`)
	return done === true
}

export async function push(): Promise<void> {
	const ahead = git.status?.ahead ?? 0
	const done = await withGit(async () => {
		await git.push()
		return true
	})
	if (done) {
		toasts.show(
			ahead
				? `Pushed ${plural(ahead, 'commit', 'commits')} to ${git.remoteName ?? 'origin'}`
				: 'Pushed',
		)
	}
}

export async function pull(): Promise<void> {
	const done = await withGit(async () => {
		await git.pull()
		return true
	})
	if (done) {
		await ws.reload()
		toasts.show(`Pulled from ${git.remoteName ?? 'origin'}`)
	}
}

export async function fetch(): Promise<void> {
	await withGit(() => git.fetch())
}

function withDependencies(picked: ItemChange[], all: ItemChange[]): ItemChange[] {
	const group = [...picked]
	const names = new Set(picked.map((c) => c.name))
	let grew = true
	while (grew) {
		grew = false
		for (const item of all) {
			if (group.includes(item) || !item.dependency || item.parents.length === 0) continue
			if (!item.parents.every((p) => names.has(p))) continue
			group.push(item)
			names.add(item.name)
			grew = true
		}
	}
	return group
}

export async function discardContent(picked: ItemChange[], ask = true): Promise<void> {
	if (picked.length === 0) return
	const group = withDependencies(picked, content.changes?.items ?? [])
	const extra = group.slice(picked.length).map((c) => c.name)
	if (!ask) {
		await withGit(() => git.discard(group.map((c) => c.path)))
		return
	}
	const ok = await dialogs.open(ConfirmDialog, {
		title:
			picked.length === 1
				? `Discard changes to ${picked[0].name}?`
				: `Discard ${picked.length} content changes?`,
		message:
			extra.length > 0
				? `Also discards the dependency changes that came with ${picked.length === 1 ? 'it' : 'them'}: ${extra.join(', ')}. This can't be undone.`
				: "This can't be undone.",
		confirm: picked.length === 1 ? 'Discard' : `Discard ${picked.length}`,
		danger: true,
	})
	if (!ok) return
	await withGit(() => git.discard(group.map((c) => c.path)))
}

export async function discard(paths: string[]): Promise<void> {
	if (paths.length === 0) return
	const ok = await dialogs.open(ConfirmDialog, {
		title:
			paths.length === 1 ? `Discard changes to ${paths[0]}?` : `Discard ${paths.length} changes?`,
		message: "This can't be undone.",
		confirm: paths.length === 1 ? 'Discard' : `Discard ${paths.length}`,
		danger: true,
	})
	if (!ok) return
	await withGit(() => git.discard(paths))
}

export async function addRemote(): Promise<void> {
	const url = await dialogs.open(PromptDialog, {
		title: 'Add a remote',
		label: 'Repository URL',
		placeholder: 'https://github.com/you/your-pack.git',
		confirm: 'Add remote',
	})
	if (!url || !ws.dir) return
	const dir = ws.dir
	const taken = new Set(git.remotes.map((r) => r.name))
	const base = taken.size === 0 ? 'origin' : 'upstream'
	let name = base
	for (let n = 2; taken.has(name); n++) name = `${base}-${n}`
	await withGit(async () => {
		await api.addRemote(dir, name, url.trim())
		await git.refreshRefs()
	})
}

export async function stashChanges(): Promise<void> {
	if (!ws.dir) return
	const dir = ws.dir
	const message = await dialogs.open(PromptDialog, {
		title: 'Stash changes',
		label: 'Name',
		fallback: 'Work in progress',
		confirm: 'Stash',
	})
	if (message === undefined) return
	await withGit(async () => {
		await api.stashPush(dir, message)
		await ws.reload()
	})
}

export async function applyStash(reference: string, drop: boolean): Promise<void> {
	if (!ws.dir) return
	const dir = ws.dir
	await withGit(async () => {
		await api.stashApply(dir, reference, drop)
		await ws.reload()
	})
}

export async function dropStash(reference: string): Promise<void> {
	if (!ws.dir) return
	const dir = ws.dir
	const ok = await dialogs.open(ConfirmDialog, {
		title: 'Delete this stash?',
		message: "This can't be undone.",
		confirm: 'Delete stash',
		danger: true,
	})
	if (!ok) return
	await withGit(async () => {
		await api.stashDrop(dir, reference)
		await git.refreshRefs()
	})
}

export async function createTag(target: string | null = null): Promise<void> {
	if (!ws.dir) return
	const dir = ws.dir
	const name = await dialogs.open(PromptDialog, {
		title: target ? 'Tag this commit' : 'New tag',
		label: 'Tag',
		fallback: `v${ws.manifest?.version ?? '1.0.0'}`,
		confirm: 'Create tag',
	})
	if (!name) return
	await withGit(async () => {
		await api.createTag(dir, name.trim(), null, target)
		await git.refreshRefs()
		toasts.show(`Tagged ${name.trim()}`)
	})
}

export async function deleteTag(name: string): Promise<void> {
	if (!ws.dir) return
	const dir = ws.dir
	const ok = await dialogs.open(ConfirmDialog, {
		title: `Delete tag ${name}?`,
		message: 'Only the local tag is deleted.',
		confirm: 'Delete tag',
		danger: true,
	})
	if (!ok) return
	await withGit(async () => {
		await api.deleteTag(dir, name)
		await git.refreshRefs()
	})
}

export async function revertCommit(hash: string): Promise<void> {
	if (!ws.dir) return
	const dir = ws.dir
	await withGit(async () => {
		await api.revertCommit(dir, hash)
		await ws.reload()
		toasts.show('Reverted the commit')
	})
}

export async function cherryPick(hash: string): Promise<void> {
	if (!ws.dir) return
	const dir = ws.dir
	await withGit(async () => {
		await api.cherryPick(dir, hash)
		await ws.reload()
		toasts.show(`Picked the commit onto ${git.branch}`)
	})
}

export async function resetTo(hash: string): Promise<void> {
	if (!ws.dir) return
	const dir = ws.dir
	const ok = await dialogs.open(ConfirmDialog, {
		title: `Reset ${git.branch} to ${hash.slice(0, 7)}?`,
		message: 'Later commits leave the branch. Files keep their changes.',
		confirm: 'Reset',
		danger: true,
	})
	if (!ok) return
	await withGit(async () => {
		await api.reset(dir, hash, 'mixed')
		await ws.reload()
	})
}

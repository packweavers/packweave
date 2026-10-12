import * as api from '@/lib/tauri/git'
import type {
	Branch,
	Branches,
	FileStat,
	GitChange,
	GitSetup,
	GitStatus,
	MergeState,
	OpenWindow,
	Remote,
	Stash,
	Tag,
	Worktree,
} from '@/lib/tauri/types'
import { packWindows } from '@/lib/tauri/windows'
import { sameDir } from '@/lib/util/path'

import { app } from './app.svelte'
import { tasks } from './tasks.svelte'
import { ws } from './workspace.svelte'

class GitState {
	setup = $state.raw<GitSetup | null>(null)
	status = $state.raw<GitStatus | null>(null)
	branches = $state.raw<Branches | null>(null)
	remotes = $state.raw<Remote[]>([])
	tags = $state.raw<Tag[]>([])
	stashes = $state.raw<Stash[]>([])
	worktrees = $state.raw<Worktree[]>([])
	windows = $state.raw<OpenWindow[]>([])
	merge = $state.raw<MergeState | null>(null)
	stats = $state.raw<Record<string, FileStat>>({})
	busy = $state<'push' | 'pull' | 'fetch' | null>(null)
	excluded = $state<Record<string, boolean>>({})
	message = $state('')
	composing = $state(false)
	focus = $state<string | null>(null)
	branch = $derived(this.status?.branch ?? null)
	head = $derived(this.status?.head ?? null)

	get isRepo(): boolean {
		return this.status?.isRepo === true
	}

	get changes(): GitChange[] {
		return this.status?.changes ?? []
	}

	get included(): string[] {
		return this.changes.filter((c) => !this.excluded[c.path] && !c.conflicted).map((c) => c.path)
	}

	include(paths: string[], on: boolean): void {
		for (const path of paths) {
			if (on) {
				delete this.excluded[path]
			} else {
				this.excluded[path] = true
			}
		}
	}

	get conflicts(): string[] {
		return this.merge?.conflicts ?? []
	}

	get merging(): boolean {
		return this.merge?.merging === true
	}

	get remoteName(): string | null {
		const names = this.remotes.map((r) => r.name)
		return names.includes('origin') ? 'origin' : (names[0] ?? null)
	}

	reset(): void {
		this.status = null
		this.branches = null
		this.remotes = []
		this.tags = []
		this.stashes = []
		this.worktrees = []
		this.merge = null
		this.stats = {}
		this.busy = null
		this.excluded = {}
		this.message = ''
	}

	async loadSetup(): Promise<GitSetup | null> {
		this.setup = await api.setup().catch(tasks.or('Checking Git', null))
		return this.setup
	}

	async refresh(): Promise<void> {
		const dir = ws.dir
		if (!dir) return
		const [status, merge, stats] = await Promise.all([
			api.status(dir).catch(tasks.or('Reading Git status', null)),
			api.mergeState(dir).catch(tasks.or('Reading the merge state', null)),
			api.workingStats(dir).catch(tasks.or('Counting changed lines', [])),
		])
		if (ws.dir !== dir) return
		this.status = status
		this.merge = merge
		this.stats = Object.fromEntries(stats.map((s) => [s.path, s]))
		ws.retitle()
	}

	async refreshRefs(): Promise<void> {
		const dir = ws.dir
		if (!dir) return
		const [branches, remotes, tags, stashes, worktrees, windows] = await Promise.all([
			api.branches(dir).catch(tasks.or('Listing branches', null)),
			api.remotes(dir).catch(tasks.or('Listing remotes', [])),
			api.tags(dir).catch(tasks.or('Listing tags', [])),
			api.stashes(dir).catch(tasks.or('Listing stashes', [])),
			api.worktrees(dir).catch(tasks.or('Listing branch folders', [])),
			packWindows().catch(tasks.or('Listing open windows', [])),
		])
		if (ws.dir !== dir) return
		this.branches = branches
		this.remotes = remotes
		this.tags = tags
		this.stashes = stashes
		this.worktrees = worktrees
		this.windows = windows
	}

	async refreshAll(): Promise<void> {
		await Promise.all([this.refresh(), this.refreshRefs(), this.setup ? null : this.loadSetup()])
	}

	get locals(): Branch[] {
		return (this.branches?.list ?? []).filter((b) => !b.remote)
	}

	get remoteOnly(): Branch[] {
		const locals = this.locals
		return (this.branches?.list ?? []).filter(
			(b) =>
				b.remote &&
				!locals.some(
					(l) => l.upstream === b.name || l.name === b.name.slice(b.name.indexOf('/') + 1),
				),
		)
	}

	isRemote(name: string): boolean {
		return (this.branches?.list ?? []).some((b) => b.remote && b.name === name)
	}

	worktreeFor(branch: string): Worktree | undefined {
		return this.worktrees.find((w) => w.branch === branch)
	}

	windowFor(folder: string): OpenWindow | undefined {
		return this.windows.find((w) => sameDir(w.dir, folder))
	}

	async init(): Promise<void> {
		if (!ws.dir) return
		await api.init(ws.dir)
		await this.refreshAll()
	}

	async commit(message: string, files: string[], amend: boolean): Promise<void> {
		const dir = ws.dir
		if (!dir) return
		await tasks.run('Committing', () => api.commit(dir, message, files, amend))
		await this.refresh()
		await this.refreshRefs()
	}

	async #network(
		kind: 'push' | 'pull' | 'fetch',
		label: string,
		work: () => Promise<unknown>,
	): Promise<void> {
		this.busy = kind
		try {
			await tasks.run(label, work)
		} finally {
			this.busy = null
			await this.refreshAll()
		}
	}

	push(): Promise<void> {
		const dir = ws.dir
		if (!dir) return Promise.resolve()
		return this.#network('push', `Pushing to ${this.remoteName ?? 'origin'}`, () => api.push(dir))
	}

	pull(): Promise<void> {
		const dir = ws.dir
		if (!dir) return Promise.resolve()
		return this.#network('pull', `Pulling from ${this.remoteName ?? 'origin'}`, () =>
			api.pull(dir, app.prefs.pullStrategy),
		)
	}

	fetch(): Promise<void> {
		const dir = ws.dir
		if (!dir) return Promise.resolve()
		return this.#network('fetch', `Fetching ${this.remoteName ?? 'origin'}`, () => api.fetch(dir))
	}

	async discard(paths: string[]): Promise<void> {
		if (!ws.dir || paths.length === 0) return
		await api.discardFiles(ws.dir, paths)
		await ws.reload()
	}
}

export const git = new GitState()

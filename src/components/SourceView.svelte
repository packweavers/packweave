<script lang="ts">
	import { onMount } from 'svelte'
	import {
		GitBranch,
		GitCompare,
		GitMerge,
		GitCommitHorizontal,
		RefreshCw,
		ArrowUp,
		ArrowDown,
		DownloadCloud,
		RotateCcw,
		Check,
		Plus,
		Trash2,
		FileText,
		History,
		Cloud,
		KeyRound,
		ChevronDown,
		Archive,
		Tag as TagIcon,
		MoreHorizontal,
		Pencil,
		Undo2,
		CornerUpLeft,
		Copy,
		Upload,
	} from '@lucide/svelte'
	import ButtonStyled from './ui/ButtonStyled.svelte'
	import Modal from './ui/Modal.svelte'
	import Dropdown from './ui/Dropdown.svelte'
	import SemanticDiff from './SemanticDiff.svelte'
	import ChangelogTab from './source/ChangelogTab.svelte'
	import { api } from '../api'
	import { store } from '../lib/store.svelte'
	import { contextMenu } from '../lib/contextmenu.svelte'
	import { tooltip } from '../lib/tooltip'
	import type {
		Branch,
		Branches,
		GitChange,
		GitCommit,
		PackDiff,
		PullStrategy,
		Remote,
		Stash,
		Tag,
	} from '../types'

	let tab = $state<'commit' | 'log' | 'changelog'>('commit')

	let checks = $state<Record<string, boolean>>({})
	let message = $state('')
	let amend = $state(false)
	let activeFile = $state<string | null>(null)
	let diffText = $state('')
	let diffLoading = $state(false)

	const hasGitToken = $derived(store.getPref('secret:git_token', false))

	let branches = $state<Branches>({ current: null, detached: false, list: [] })
	let remotes = $state<Remote[]>([])
	let commits = $state<GitCommit[]>([])
	let stashes = $state<Stash[]>([])
	let tags = $state<Tag[]>([])

	const localBranches = $derived(branches.list.filter((b) => !b.remote))
	const remoteBranches = $derived(branches.list.filter((b) => b.remote))

	let newBranch = $state('')
	let bExpand = $state<Record<string, boolean>>({})
	let renaming = $state<string | null>(null)
	let renameValue = $state('')
	let confirmBranch = $state<string | null>(null)
	let confirmRemoteBranch = $state<string | null>(null)

	let confirmRemote = $state<string | null>(null)
	let newRemoteName = $state('origin')
	let newRemoteUrl = $state('')

	let stashMsg = $state('')
	let stashUntracked = $state(true)

	let newTagName = $state('')
	let newTagMsg = $state('')
	let confirmTag = $state<string | null>(null)

	let activeCommit = $state<string | null>(null)
	let commitFiles = $state<GitChange[]>([])
	let packDiff = $state<PackDiff | null>(null)
	let workingDiff = $state<PackDiff | null>(null)

	let compareBranch = $state<string | null>(null)
	let compareDiff = $state<PackDiff | null>(null)
	let compareLoading = $state(false)

	let ignoreOpen = $state(false)
	let ignoreContent = $state('')
	let ignoreDirty = $state(false)

	const changes = $derived(store.git?.changes ?? [])
	const isSel = (p: string) => checks[p] !== false
	const selected = $derived(changes.filter((c) => isSel(c.path)).map((c) => c.path))
	const allSel = $derived(changes.length > 0 && changes.every((c) => isSel(c.path)))

	function toggle(p: string) {
		checks[p] = !isSel(p)
	}
	function toggleAll() {
		const v = !allSel
		for (const c of changes) checks[c.path] = v
	}

	const STATUS_LABEL: Record<string, string> = {
		A: 'added',
		'?': 'new',
		D: 'deleted',
		M: 'modified',
		R: 'renamed',
		C: 'copied',
		U: 'conflict',
	}
	function statusKind(s: string): string {
		if (s === 'U') return 'conflict'
		if (s === 'A' || s === '?') return 'add'
		if (s === 'D') return 'del'
		return 'mod'
	}

	async function loadAux() {
		if (!store.pack) return
		const dir = store.pack.dir
		const [b, r, c, s, t, ig] = await Promise.allSettled([
			api.gitBranches(dir),
			api.gitRemotes(dir),
			api.gitLog(dir, 100),
			api.gitStashList(dir),
			api.gitTags(dir),
			api.readGitignore(dir),
		])
		branches = b.status === 'fulfilled' ? b.value : { current: null, detached: false, list: [] }
		remotes = r.status === 'fulfilled' ? r.value : []
		commits = c.status === 'fulfilled' ? c.value : []
		stashes = s.status === 'fulfilled' ? s.value : []
		tags = t.status === 'fulfilled' ? t.value : []
		if (ig.status === 'fulfilled') ignoreContent = ig.value
		try {
			workingDiff = await api.gitPackDiffWorking(dir)
		} catch {
			workingDiff = null
		}
	}

	onMount(async () => {
		await store.refreshGit()
		await loadAux()
	})

	function resetDiff() {
		activeFile = null
		activeCommit = null
		commitFiles = []
		diffText = ''
		packDiff = null
	}

	async function reload(treeChanged = false) {
		await loadAux()
		if (treeChanged) resetDiff()
	}

	async function openWorkingDiff(c: GitChange) {
		if (!store.pack) return
		activeFile = c.path
		diffLoading = true
		try {
			diffText = await api.gitDiffFile(store.pack.dir, c.path, false)
		} catch (e) {
			diffText = `error: ${e}`
		} finally {
			diffLoading = false
		}
	}

	async function openCommit(hash: string) {
		if (!store.pack) return
		activeCommit = hash
		activeFile = null
		diffText = ''
		packDiff = null
		try {
			commitFiles = await api.gitCommitChanges(store.pack.dir, hash)
		} catch {
			commitFiles = []
		}
		try {
			packDiff = await api.gitPackDiff(store.pack.dir, `${hash}^`, hash)
		} catch {
			packDiff = null
		}
	}

	async function compare(branch: string) {
		if (!store.pack) return
		compareBranch = branch
		compareDiff = null
		compareLoading = true
		try {
			compareDiff = await api.gitPackDiff(store.pack.dir, branches.current ?? 'HEAD', branch)
		} catch (e) {
			store.notify('error', `${e}`)
			compareBranch = null
		} finally {
			compareLoading = false
		}
	}
	async function mergeFromCompare() {
		const b = compareBranch
		compareBranch = null
		if (b) await merge(b)
	}

	async function openCommitFile(file: string) {
		if (!store.pack || !activeCommit) return
		activeFile = file
		diffLoading = true
		try {
			diffText = await api.gitShowDiff(store.pack.dir, activeCommit, file)
		} catch (e) {
			diffText = `error: ${e}`
		} finally {
			diffLoading = false
		}
	}

	async function doCommit() {
		await store.gitCommit(message, selected, amend)
		message = ''
		amend = false
		await reload(true)
	}
	async function doCommitPush() {
		await store.gitCommitPush(message, selected, amend)
		message = ''
		amend = false
		await reload(true)
	}
	async function revert(path: string) {
		await store.gitRevert([path])
		if (activeFile === path) resetDiff()
		await reload()
	}
	async function revertSelected() {
		await store.gitRevert(selected)
		await reload(true)
	}
	async function resolveConflict(path: string, side: 'ours' | 'theirs') {
		await store.gitResolveConflict(path, side)
		if (activeFile === path) resetDiff()
		await reload()
	}

	async function checkout(name: string) {
		await store.gitCheckout(name)
		await reload(true)
	}
	async function createBranch(checkoutIt: boolean) {
		const name = newBranch.trim()
		if (!name) return
		await store.gitCreateBranch(name, undefined, checkoutIt)
		newBranch = ''
		await reload(checkoutIt)
	}
	async function merge(name: string) {
		await store.gitMerge(name)
		await reload(true)
	}
	async function rebase(name: string) {
		await store.gitRebase(name)
		await reload(true)
	}
	function startRename(b: Branch) {
		renaming = b.name
		renameValue = b.name
	}
	async function saveRename(oldName: string) {
		const next = renameValue.trim()
		renaming = null
		if (!next || next === oldName) return
		await store.gitRenameBranch(oldName, next)
		await reload()
	}
	async function delLocal(name: string) {
		confirmBranch = null
		await store.gitDeleteBranch(name, true)
		await reload()
	}
	async function delRemote(name: string) {
		confirmRemoteBranch = null
		await store.gitDeleteRemoteBranch(name)
		await reload()
	}
	async function pushBranch(b: Branch) {
		const remote = remotes[0]?.name ?? 'origin'
		await store.gitPushBranch(remote, b.name)
		await reload()
	}
	async function setUpstream(upstream: string) {
		await store.gitSetUpstream(upstream)
		await reload()
	}

	async function fetch() {
		await store.gitFetch()
		await reload()
	}
	async function pull(strategy: PullStrategy) {
		await store.gitPull(strategy)
		await reload(true)
	}
	async function push(force: boolean, withTags: boolean) {
		await store.gitPush(force, withTags)
		await reload()
	}

	async function addRemote() {
		const name = newRemoteName.trim()
		const url = newRemoteUrl.trim()
		if (!store.pack || !name || !url) return
		try {
			if (remotes.some((r) => r.name === name)) {
				await api.gitSetRemoteUrl(store.pack.dir, name, url)
			} else {
				await api.gitAddRemote(store.pack.dir, name, url)
			}
			newRemoteUrl = ''
			await store.refreshGit()
			await loadAux()
		} catch (e) {
			store.notify('error', `${e}`)
		}
	}
	async function removeRemote(name: string) {
		if (!store.pack) return
		confirmRemote = null
		try {
			await api.gitRemoveRemote(store.pack.dir, name)
			await store.refreshGit()
			await loadAux()
		} catch (e) {
			store.notify('error', `${e}`)
		}
	}

	async function stashCreate() {
		await store.gitStash(stashMsg, stashUntracked)
		stashMsg = ''
		await reload(true)
	}
	async function stashApply(reference: string, drop: boolean) {
		await store.gitStashApply(reference, drop)
		await reload(true)
	}
	async function stashDrop(reference: string) {
		await store.gitStashDrop(reference)
		await loadAux()
	}

	async function createTag(target?: string) {
		const name = newTagName.trim()
		if (!name) return
		await store.gitCreateTag(name, newTagMsg || undefined, target)
		newTagName = ''
		newTagMsg = ''
		await loadAux()
	}
	async function deleteTag(name: string) {
		confirmTag = null
		await store.gitDeleteTag(name)
		await loadAux()
	}
	async function pushTag(name: string) {
		await store.gitPushTag(name)
	}

	async function revertCommit(hash: string) {
		await store.gitRevertCommit(hash)
		await reload(true)
	}
	async function resetTo(hash: string, mode: 'soft' | 'mixed' | 'hard') {
		await store.gitReset(hash, mode)
		await reload(true)
	}
	async function cherryPick(hash: string) {
		await store.gitCherryPick(hash)
		await reload(true)
	}
	async function branchFrom(hash: string) {
		const name = window.prompt('New branch from this commit:')?.trim()
		if (!name) return
		await store.gitCreateBranch(name, hash, true)
		await reload(true)
	}
	async function tagAt(hash: string) {
		const name = window.prompt('Tag name (e.g. v1.0.0):')?.trim()
		if (!name) return
		await store.gitCreateTag(name, undefined, hash)
		await loadAux()
	}
	function copyHash(hash: string) {
		navigator.clipboard?.writeText(hash)
		store.notify('success', 'Copied commit hash')
	}

	function openChangelog() {
		tab = 'changelog'
	}

	async function saveIgnore() {
		if (!store.pack) return
		try {
			await api.writeGitignore(store.pack.dir, ignoreContent)
			ignoreDirty = false
			store.notify('success', 'Saved .gitignore')
			await store.refreshSync()
			await store.refreshGit()
		} catch (e) {
			store.notify('error', `${e}`)
		}
	}

	interface Side {
		n: number | null
		text: string
		cls: 'ctx' | 'del' | 'add' | 'none'
	}
	const rows = $derived.by<{ l: Side; r: Side }[]>(() => {
		const out: { l: Side; r: Side }[] = []
		let oldLn = 1
		let newLn = 1
		let dels: Side[] = []
		let adds: Side[] = []
		const flush = () => {
			const n = Math.max(dels.length, adds.length)
			for (let i = 0; i < n; i++) {
				out.push({
					l: dels[i] ?? { n: null, text: '', cls: 'none' },
					r: adds[i] ?? { n: null, text: '', cls: 'none' },
				})
			}
			dels = []
			adds = []
		}
		for (const raw of diffText.split('\n')) {
			if (raw.startsWith('@@')) {
				flush()
				const m = /@@ -(\d+)(?:,\d+)? \+(\d+)(?:,\d+)? @@/.exec(raw)
				if (m) {
					oldLn = parseInt(m[1], 10)
					newLn = parseInt(m[2], 10)
				}
				continue
			}
			if (
				raw.startsWith('+++') ||
				raw.startsWith('---') ||
				raw.startsWith('diff ') ||
				raw.startsWith('index ') ||
				raw.startsWith('new file') ||
				raw.startsWith('deleted file') ||
				raw.startsWith('similarity ') ||
				raw.startsWith('rename ') ||
				raw.startsWith('old mode') ||
				raw.startsWith('new mode') ||
				raw.startsWith('\\')
			) {
				continue
			}
			const c = raw[0]
			const text = raw.slice(1)
			if (c === '-') {
				dels.push({ n: oldLn++, text, cls: 'del' })
			} else if (c === '+') {
				adds.push({ n: newLn++, text, cls: 'add' })
			} else {
				flush()
				out.push({ l: { n: oldLn++, text, cls: 'ctx' }, r: { n: newLn++, text, cls: 'ctx' } })
			}
		}
		flush()
		return out
	})

	const sideBg: Record<Side['cls'], string> = {
		del: 'bg-[color-mix(in_srgb,var(--color-red)_13%,transparent)]',
		add: 'bg-[color-mix(in_srgb,var(--color-green)_13%,transparent)]',
		none: 'bg-bg-inset',
		ctx: '',
	}
	const stCls: Record<string, string> = {
		add: 'bg-[color-mix(in_srgb,var(--color-green)_18%,transparent)] text-green',
		del: 'bg-[color-mix(in_srgb,var(--color-red)_16%,transparent)] text-red',
		mod: 'bg-button-bg text-secondary',
		conflict: 'bg-[color-mix(in_srgb,var(--color-orange)_20%,transparent)] text-orange',
	}

	const scBtn =
		'inline-flex items-center gap-[0.2rem] bg-transparent border-none text-secondary min-w-[1.9rem] h-[1.9rem] justify-center px-[0.35rem] rounded-sm cursor-pointer enabled:hover:bg-button-bg enabled:hover:text-contrast disabled:opacity-40 disabled:cursor-default'
	const scCaret =
		'bg-transparent border-none text-secondary h-[1.9rem] w-4 grid place-items-center cursor-pointer rounded-sm enabled:hover:bg-button-bg enabled:hover:text-contrast disabled:opacity-40 disabled:cursor-default'
	const miniBtn =
		'grid place-items-center w-[1.7rem] h-[1.7rem] border border-divider bg-button-bg text-body rounded-sm cursor-pointer shrink-0 enabled:hover:text-contrast disabled:opacity-40 disabled:cursor-default'
	const mItem =
		'flex items-center gap-1.5 bg-transparent border-none text-body text-[0.8rem] text-left px-2 py-[0.4rem] rounded-sm cursor-pointer enabled:hover:bg-button-bg enabled:hover:text-contrast disabled:opacity-40 disabled:cursor-default'
	const menuLabel =
		'text-[0.64rem] uppercase tracking-[0.05em] text-secondary font-[650] px-[0.3rem] pt-[0.3rem] pb-[0.15rem]'
	const mInput =
		'w-full bg-bg-inset border border-divider text-contrast rounded-sm px-2 py-[0.35rem] text-[0.8rem] outline-none mb-[0.3rem]'
	const baBtn =
		'inline-flex items-center gap-1 bg-bg-inset border border-divider text-body text-[0.72rem] px-[0.45rem] py-[0.2rem] rounded-sm cursor-pointer hover:text-contrast hover:border-divider-dark'
	const bnameBtn =
		'flex-1 flex items-center gap-[0.45rem] bg-transparent border-none text-body text-[0.82rem] text-left px-2 py-[0.4rem] rounded-sm cursor-pointer min-w-0 enabled:hover:bg-button-bg enabled:hover:text-contrast disabled:cursor-default'
	const trk = 'inline-flex items-center gap-[0.05rem] text-[0.62rem] text-secondary'
	const sName =
		'text-[0.8rem] font-semibold text-contrast whitespace-nowrap overflow-hidden text-ellipsis'
	const sSub = 'text-[0.68rem] text-secondary whitespace-nowrap overflow-hidden text-ellipsis'
	const yesBtn = 'bg-transparent border-none text-red text-[0.72rem] cursor-pointer'
	const noBtn = 'bg-transparent border-none text-secondary text-[0.72rem] cursor-pointer'
	const menuDivider = 'h-px bg-divider my-1'
</script>

<div class="h-full flex flex-col min-h-0">
	{#if !store.git}
		<div class="flex-1 flex flex-col items-center justify-center gap-[0.6rem] text-secondary">
			Checking repository…
		</div>
	{:else if !store.git.isRepo}
		<div
			class="flex-1 flex flex-col items-center justify-center gap-[0.6rem] text-secondary text-center"
		>
			<GitBranch size={30} />
			<p>This pack isn't a Git repository yet.</p>
			<ButtonStyled color="brand" disabled={store.busy} onclick={() => store.gitInit().then(loadAux)}>
				<GitBranch size={15} /> Initialize repository
			</ButtonStyled>
		</div>
	{:else}
		<header class="flex items-center gap-[0.6rem] px-4 py-[0.6rem] border-b border-divider">
			<Dropdown placement="bottom-start">
				{#snippet trigger()}
					<button
						class="inline-flex items-center gap-[0.4rem] bg-bg-inset border border-divider text-contrast px-[0.65rem] py-[0.35rem] rounded-md text-[0.82rem] font-[550] cursor-pointer hover:border-divider-dark"
					>
						<GitBranch size={14} />
						<span>{branches.current ?? (store.git!.detached ? 'detached HEAD' : 'no branch')}</span>
						{#if store.git!.behind}
							<span class="inline-flex items-center gap-[0.05rem] text-[0.68rem] text-secondary"
								><ArrowDown size={11} />{store.git!.behind}</span
							>
						{/if}
						{#if store.git!.ahead}
							<span class="inline-flex items-center gap-[0.05rem] text-[0.68rem] text-secondary"
								><ArrowUp size={11} />{store.git!.ahead}</span
							>
						{/if}
						<ChevronDown size={12} />
					</button>
				{/snippet}
				{#snippet content(hide)}
					<div class="w-[280px]">
						<div class="flex gap-[0.35rem] px-[0.2rem] pt-[0.2rem] pb-[0.45rem] border-b border-divider mb-[0.3rem]">
							<input
								bind:value={newBranch}
								class="flex-1 bg-bg-inset border border-divider text-contrast rounded-sm px-2 py-[0.3rem] text-[0.8rem] outline-none"
								placeholder="New branch…"
								spellcheck="false"
								onkeydown={(e) => e.key === 'Enter' && createBranch(true).then(hide)}
							/>
							<button
								class={miniBtn}
								use:tooltip={'Create'}
								aria-label="Create"
								disabled={!newBranch.trim()}
								onclick={() => createBranch(true).then(hide)}
							>
								<Plus size={14} />
							</button>
						</div>

						<div class={menuLabel}>Local</div>
						<div class="flex flex-col gap-px max-h-[38vh] overflow-y-auto">
							{#each localBranches as b (b.name)}
								<div class="flex items-center gap-[0.3rem] flex-wrap">
									{#if renaming === b.name}
										<input
											bind:value={renameValue}
											class="flex-1 bg-bg-inset border border-divider text-contrast rounded-sm px-2 py-[0.3rem] text-[0.8rem] outline-none"
											spellcheck="false"
											onkeydown={(e) => {
												if (e.key === 'Enter') saveRename(b.name)
												else if (e.key === 'Escape') renaming = null
											}}
										/>
										<button class={miniBtn} onclick={() => saveRename(b.name)}><Check size={13} /></button>
									{:else}
										<button
											class="{bnameBtn} {b.current ? 'text-contrast font-semibold' : ''}"
											disabled={b.current}
											use:contextMenu={() => [
												!b.current && {
													label: 'Checkout',
													icon: GitBranch,
													onSelect: () => checkout(b.name).then(hide),
												},
												!b.current && {
													label: `Merge into ${branches.current}`,
													icon: GitMerge,
													onSelect: () => merge(b.name).then(hide),
												},
												!b.current && {
													label: `Rebase ${branches.current} onto`,
													onSelect: () => rebase(b.name).then(hide),
												},
												{ label: 'Push', icon: Upload, onSelect: () => pushBranch(b) },
												{ label: 'Rename', icon: Pencil, onSelect: () => startRename(b) },
												!b.current && { separator: true },
												!b.current && {
													label: 'Delete',
													icon: Trash2,
													danger: true,
													onSelect: () => delLocal(b.name).then(hide),
												},
											]}
											onclick={() => b.current || checkout(b.name).then(hide)}
										>
											{#if b.current}<Check size={13} />{:else}<GitBranch size={13} />{/if}
											<span class="flex-1 whitespace-nowrap overflow-hidden text-ellipsis">{b.name}</span>
											{#if b.behind}<span class={trk}><ArrowDown size={10} />{b.behind}</span>{/if}
											{#if b.ahead}<span class={trk}><ArrowUp size={10} />{b.ahead}</span>{/if}
											{#if b.gone}<span class="{trk} text-red">gone</span>{/if}
										</button>
										{#if !b.current}
											<button
												class={miniBtn}
												use:tooltip={'Compare with current branch'}
												aria-label="Compare with current branch"
												onclick={() => {
													compare(b.name)
													hide()
												}}
											>
												<GitCompare size={13} />
											</button>
										{/if}
										<button
											class={miniBtn}
											use:tooltip={'More'}
											aria-label="More"
											onclick={() => (bExpand[b.name] = !bExpand[b.name])}
										>
											<MoreHorizontal size={14} />
										</button>
									{/if}
									{#if bExpand[b.name] && renaming !== b.name}
										<div class="w-full flex flex-wrap gap-[0.25rem] pt-[0.2rem] pr-[0.3rem] pb-[0.4rem] pl-[1.8rem]">
											{#if !b.current}
												<button class={baBtn} onclick={() => checkout(b.name).then(hide)}>Checkout</button>
											{/if}
											{#if !b.current}
												<button class={baBtn} onclick={() => merge(b.name).then(hide)}>
													<GitMerge size={12} /> Merge into {branches.current}
												</button>
											{/if}
											{#if !b.current}
												<button class={baBtn} onclick={() => rebase(b.name).then(hide)}
													>Rebase {branches.current} onto</button
												>
											{/if}
											<button class={baBtn} onclick={() => pushBranch(b)}>Push</button>
											<button class={baBtn} onclick={() => startRename(b)}><Pencil size={12} /> Rename</button>
											{#if confirmBranch === b.name}
												<span class="inline-flex items-center gap-[0.3rem]">
													<button class={yesBtn} onclick={() => delLocal(b.name).then(hide)}>Delete</button>
													<button class={noBtn} onclick={() => (confirmBranch = null)}>No</button>
												</span>
											{:else if !b.current}
												<button
													class="{baBtn} hover:!text-red hover:!border-red"
													onclick={() => (confirmBranch = b.name)}
												>
													<Trash2 size={12} /> Delete
												</button>
											{/if}
										</div>
									{/if}
								</div>
							{/each}
						</div>

						{#if remoteBranches.length}
							<div class={menuLabel}>Remote</div>
							<div class="flex flex-col gap-px max-h-[38vh] overflow-y-auto">
								{#each remoteBranches as b (b.name)}
									<div class="flex items-center gap-[0.3rem] flex-wrap">
										<button
											class={bnameBtn}
											use:contextMenu={() => [
												{
													label: 'Checkout (track)',
													icon: GitBranch,
													onSelect: () => checkout(b.name).then(hide),
												},
												{
													label: `Merge into ${branches.current}`,
													icon: GitMerge,
													onSelect: () => merge(b.name).then(hide),
												},
												!!branches.current && {
													label: 'Track as upstream',
													onSelect: () => setUpstream(b.name),
												},
												{ separator: true },
												{
													label: 'Delete on remote',
													icon: Trash2,
													danger: true,
													onSelect: () => delRemote(b.name).then(hide),
												},
											]}
											onclick={() => checkout(b.name).then(hide)}
										>
											<Cloud size={13} />
											<span class="flex-1 whitespace-nowrap overflow-hidden text-ellipsis">{b.name}</span>
										</button>
										<button
											class={miniBtn}
											use:tooltip={'Compare with current branch'}
											aria-label="Compare with current branch"
											onclick={() => {
												compare(b.name)
												hide()
											}}
										>
											<GitCompare size={13} />
										</button>
										<button
											class={miniBtn}
											use:tooltip={'More'}
											aria-label="More"
											onclick={() => (bExpand[b.name] = !bExpand[b.name])}
										>
											<MoreHorizontal size={14} />
										</button>
										{#if bExpand[b.name]}
											<div class="w-full flex flex-wrap gap-[0.25rem] pt-[0.2rem] pr-[0.3rem] pb-[0.4rem] pl-[1.8rem]">
												<button class={baBtn} onclick={() => checkout(b.name).then(hide)}>Checkout (track)</button>
												<button class={baBtn} onclick={() => merge(b.name).then(hide)}
													><GitMerge size={12} /> Merge into {branches.current}</button
												>
												{#if branches.current}
													<button class={baBtn} onclick={() => setUpstream(b.name)}>Track as upstream</button>
												{/if}
												{#if confirmRemoteBranch === b.name}
													<span class="inline-flex items-center gap-[0.3rem]">
														<button class={yesBtn} onclick={() => delRemote(b.name).then(hide)}
															>Delete on remote</button
														>
														<button class={noBtn} onclick={() => (confirmRemoteBranch = null)}>No</button>
													</span>
												{:else}
													<button
														class="{baBtn} hover:!text-red hover:!border-red"
														onclick={() => (confirmRemoteBranch = b.name)}
													>
														<Trash2 size={12} /> Delete on remote
													</button>
												{/if}
											</div>
										{/if}
									</div>
								{/each}
							</div>
						{/if}
					</div>
				{/snippet}
			</Dropdown>

			<div class="flex items-center gap-[0.15rem]">
				<button
					class={scBtn}
					use:tooltip={'Fetch all remotes'}
					aria-label="Fetch all remotes"
					disabled={store.busy || !store.git.hasRemote}
					onclick={fetch}
				>
					<DownloadCloud size={15} />
				</button>

				<div class="inline-flex items-center">
					<button
						class={scBtn}
						disabled={store.busy || !store.git.hasRemote}
						use:tooltip={'Pull (fast-forward)'}
						aria-label="Pull (fast-forward)"
						onclick={() => pull('ff')}
					>
						<ArrowDown size={15} />{#if store.git.behind}<span
								class="text-[0.66rem] font-semibold">{store.git.behind}</span
							>{/if}
					</button>
					<Dropdown placement="bottom-end">
						{#snippet trigger()}
							<button class={scCaret} disabled={store.busy || !store.git!.hasRemote}
								><ChevronDown size={12} /></button
							>
						{/snippet}
						{#snippet content(hide)}
							<div class="min-w-[190px] flex flex-col gap-px">
								<button class={mItem} onclick={() => pull('ff').then(hide)}>Pull (fast-forward only)</button>
								<button class={mItem} onclick={() => pull('merge').then(hide)}>Pull (merge)</button>
								<button class={mItem} onclick={() => pull('rebase').then(hide)}>Pull (rebase)</button>
							</div>
						{/snippet}
					</Dropdown>
				</div>

				<div class="inline-flex items-center">
					<button
						class={scBtn}
						disabled={store.busy || !store.git.hasRemote}
						use:tooltip={'Push'}
						aria-label="Push"
						onclick={() => push(false, false)}
					>
						<ArrowUp size={15} />{#if store.git.ahead}<span
								class="text-[0.66rem] font-semibold">{store.git.ahead}</span
							>{/if}
					</button>
					<Dropdown placement="bottom-end">
						{#snippet trigger()}
							<button class={scCaret} disabled={store.busy || !store.git!.hasRemote}
								><ChevronDown size={12} /></button
							>
						{/snippet}
						{#snippet content(hide)}
							<div class="min-w-[190px] flex flex-col gap-px">
								<button class={mItem} onclick={() => push(false, false).then(hide)}>Push</button>
								<button class={mItem} onclick={() => push(false, true).then(hide)}>Push with tags</button>
								<button
									class="{mItem} hover:!text-red"
									onclick={() => push(true, false).then(hide)}>Force push (with lease)</button
								>
							</div>
						{/snippet}
					</Dropdown>
				</div>

				<Dropdown placement="bottom-end">
					{#snippet trigger()}
						<button class={scBtn} use:tooltip={'Stash'} aria-label="Stash"
							><Archive size={15} />{#if stashes.length}<span
									class="text-[0.66rem] font-semibold">{stashes.length}</span
								>{/if}</button
						>
					{/snippet}
					{#snippet content(hide)}
						<div class="w-[280px] flex flex-col gap-px">
							<div class={menuLabel}>Stash changes</div>
							<input bind:value={stashMsg} class={mInput} placeholder="Message (optional)" spellcheck="false" />
							<label class="flex items-center gap-[0.4rem] text-[0.76rem] text-secondary px-[0.2rem] py-[0.1rem] cursor-pointer"
								><input type="checkbox" bind:checked={stashUntracked} /> Include untracked</label
							>
							<button
								class="bg-brand text-on-brand justify-center font-semibold mt-[0.3rem] flex items-center gap-1.5 border-none text-[0.8rem] px-2 py-[0.4rem] rounded-sm cursor-pointer enabled:hover:bg-brand-hover disabled:opacity-40 disabled:cursor-default"
								disabled={store.busy || store.git!.clean}
								onclick={() => stashCreate().then(hide)}
							>
								<Archive size={13} /> Stash
							</button>
							{#if stashes.length}
								<div class={menuDivider}></div>
								<div class={menuLabel}>Stashes</div>
								{#each stashes as s (s.reference)}
									<div class="flex items-center gap-[0.3rem] px-[0.2rem] py-[0.3rem]">
										<div class="flex-1 min-w-0">
											<div class={sName}>{s.message}</div>
											<div class={sSub}>{s.branch} · {s.relative}</div>
										</div>
										<button class={miniBtn} use:tooltip={'Apply'} aria-label="Apply" onclick={() => stashApply(s.reference, false)}
											><CornerUpLeft size={12} /></button
										>
										<button class={miniBtn} use:tooltip={'Pop'} aria-label="Pop" onclick={() => stashApply(s.reference, true).then(hide)}
											><ArrowUp size={12} /></button
										>
										<button
											class="{miniBtn} hover:!text-red"
											use:tooltip={'Drop'}
											aria-label="Drop"
											onclick={() => stashDrop(s.reference)}><Trash2 size={12} /></button
										>
									</div>
								{/each}
							{/if}
						</div>
					{/snippet}
				</Dropdown>

				<Dropdown placement="bottom-end">
					{#snippet trigger()}
						<button class={scBtn} use:tooltip={'Tags'} aria-label="Tags"><TagIcon size={15} /></button>
					{/snippet}
					{#snippet content(hide)}
						<div class="w-[280px] flex flex-col gap-px">
							<div class={menuLabel}>New tag</div>
							<input
								bind:value={newTagName}
								class={mInput}
								placeholder={`v${store.pack?.manifest.version ?? '1.0.0'}`}
								spellcheck="false"
								onkeydown={(e) => e.key === 'Enter' && createTag()}
							/>
							<input
								bind:value={newTagMsg}
								class={mInput}
								placeholder="Message (annotated, optional)"
								spellcheck="false"
							/>
							<button
								class="bg-brand text-on-brand justify-center font-semibold mt-[0.3rem] flex items-center gap-1.5 border-none text-[0.8rem] px-2 py-[0.4rem] rounded-sm cursor-pointer enabled:hover:bg-brand-hover disabled:opacity-40 disabled:cursor-default"
								disabled={!newTagName.trim()}
								onclick={() => createTag()}
							>
								<TagIcon size={13} /> Create tag
							</button>
							{#if tags.length}
								<div class={menuDivider}></div>
								<div class={menuLabel}>Tags</div>
								{#each tags as t (t.name)}
									<div class="flex items-center gap-[0.3rem] px-[0.2rem] py-[0.3rem]">
										<div class="flex-1 min-w-0">
											<div class={sName}>{t.name}</div>
											{#if t.subject}<div class={sSub}>{t.subject}</div>{/if}
										</div>
										<button
											class={miniBtn}
											use:tooltip={'Push'}
											aria-label="Push"
											disabled={!store.git!.hasRemote}
											onclick={() => pushTag(t.name)}><Upload size={12} /></button
										>
										{#if confirmTag === t.name}
											<span class="inline-flex items-center gap-[0.3rem]">
												<button class={yesBtn} onclick={() => deleteTag(t.name)}>Delete</button>
												<button class={noBtn} onclick={() => (confirmTag = null)}>No</button>
											</span>
										{:else}
											<button
												class="{miniBtn} hover:!text-red"
												use:tooltip={'Delete'}
												aria-label="Delete"
												onclick={() => (confirmTag = t.name)}><Trash2 size={12} /></button
											>
										{/if}
									</div>
								{/each}
							{/if}
						</div>
					{/snippet}
				</Dropdown>

				<button class={scBtn} use:tooltip={'Refresh'} aria-label="Refresh" disabled={store.busy} onclick={() => store.refreshGit().then(loadAux)}>
					<RefreshCw size={15} class={store.busy ? 'animate-spin' : ''} />
				</button>

				<Dropdown placement="bottom-end">
					{#snippet trigger()}
						<button class={scBtn} use:tooltip={'Remotes'} aria-label="Remotes"><Cloud size={15} /></button>
					{/snippet}
					{#snippet content()}
						<div class="w-[280px] flex flex-col gap-px">
							<div class={menuLabel}>Remotes</div>
							{#if !remotes.length}
								<div class="text-[0.78rem] text-secondary px-[0.3rem] py-[0.3rem]">No remote yet.</div>
							{/if}
							{#each remotes as r (r.name)}
								<div class="flex items-center gap-[0.3rem] px-[0.2rem] py-[0.3rem]">
									<div class="flex-1 min-w-0">
										<div class={sName}>{r.name}</div>
										<div class={sSub}>{r.url}</div>
									</div>
									{#if confirmRemote === r.name}
										<span class="inline-flex items-center gap-[0.3rem]">
											<button class={yesBtn} onclick={() => removeRemote(r.name)}>Remove</button>
											<button class={noBtn} onclick={() => (confirmRemote = null)}>No</button>
										</span>
									{:else}
										<button
											class="{miniBtn} hover:!text-red"
											use:tooltip={'Remove'}
											aria-label="Remove"
											onclick={() => (confirmRemote = r.name)}><Trash2 size={13} /></button
										>
									{/if}
								</div>
							{/each}
							<div class="flex gap-[0.3rem] border-t border-divider pt-[0.4rem] mt-[0.3rem]">
								<input
									bind:value={newRemoteName}
									class="w-[4.5rem] bg-bg-inset border border-divider text-contrast rounded-sm px-[0.4rem] py-[0.3rem] text-[0.78rem] outline-none"
									placeholder="name"
									spellcheck="false"
								/>
								<input
									bind:value={newRemoteUrl}
									class="flex-1 min-w-0 bg-bg-inset border border-divider text-contrast rounded-sm px-[0.4rem] py-[0.3rem] text-[0.78rem] outline-none"
									placeholder="https://github.com/you/pack.git"
									spellcheck="false"
									onkeydown={(e) => e.key === 'Enter' && addRemote()}
								/>
								<button
									class={miniBtn}
									disabled={!newRemoteName.trim() || !newRemoteUrl.trim()}
									onclick={addRemote}><Plus size={14} /></button
								>
							</div>
							<div class="flex items-center gap-[0.4rem] mt-2 pt-2 border-t border-divider text-[0.74rem] text-secondary">
								<KeyRound size={13} class={hasGitToken ? 'text-green' : ''} />
								<span>{hasGitToken ? 'Access token set' : 'Private repos need a token'}</span>
								<button
									class="ml-auto bg-transparent border-none text-body font-semibold text-[0.74rem] cursor-pointer px-[0.3rem] py-[0.15rem] rounded-sm hover:bg-button-bg hover:text-contrast"
									onclick={() => (store.settingsOpen = true)}>Settings</button
								>
							</div>
						</div>
					{/snippet}
				</Dropdown>
			</div>

			<div class="flex gap-[0.2rem] ml-auto">
				<button
					class="inline-flex items-center gap-[0.35rem] bg-transparent border-none text-secondary text-[0.82rem] font-[550] px-[0.7rem] py-[0.35rem] rounded-md cursor-pointer hover:bg-button-bg hover:text-contrast {tab ===
					'commit'
						? 'bg-button-bg text-contrast !font-semibold'
						: ''}"
					onclick={() => {
						tab = 'commit'
						resetDiff()
					}}
				>
					Changes {#if changes.length}<span
							class="bg-brand text-on-brand rounded-max text-[0.6rem] px-[0.35rem] py-[0.05rem]"
							>{changes.length}</span
						>{/if}
				</button>
				<button
					class="inline-flex items-center gap-[0.35rem] bg-transparent border-none text-secondary text-[0.82rem] font-[550] px-[0.7rem] py-[0.35rem] rounded-md cursor-pointer hover:bg-button-bg hover:text-contrast {tab ===
					'log'
						? 'bg-button-bg text-contrast !font-semibold'
						: ''}"
					onclick={() => {
						tab = 'log'
						resetDiff()
					}}>History</button
				>
				<button
					class="inline-flex items-center gap-[0.35rem] bg-transparent border-none text-secondary text-[0.82rem] font-[550] px-[0.7rem] py-[0.35rem] rounded-md cursor-pointer hover:bg-button-bg hover:text-contrast {tab ===
					'changelog'
						? 'bg-button-bg text-contrast !font-semibold'
						: ''}"
					onclick={openChangelog}>Changelog</button
				>
			</div>
		</header>

		{#if store.git.conflicts}
			<div
				class="px-4 py-[0.45rem] bg-[color-mix(in_srgb,var(--color-red)_12%,transparent)] text-red text-[0.78rem] border-b border-divider"
			>
				{store.git.conflicts} conflicted {store.git.conflicts === 1 ? 'file' : 'files'}. Resolve them, then commit.
			</div>
		{/if}

		<div class="flex-1 grid grid-cols-[minmax(280px,360px)_1fr] min-h-0">
			{#if tab === 'commit'}
				<div class="flex flex-col border-r border-divider min-h-0">
					{#if workingDiff && (workingDiff.items.length || workingDiff.env)}
						<div class="border-b border-divider px-[0.5rem] pt-[0.45rem] pb-[0.55rem] max-h-[40%] overflow-y-auto shrink-0">
							<div class="text-[0.66rem] uppercase tracking-[0.05em] text-secondary font-[650] px-[0.2rem] pt-[0.1rem] pb-[0.35rem]">
								Uncommitted pack changes
							</div>
							<SemanticDiff diff={workingDiff} />
						</div>
					{/if}
					{#if !changes.length}
						<div class="flex flex-col items-center gap-[0.4rem] text-secondary text-center px-4 py-8 text-[0.84rem]">
							<Check size={22} /> Nothing to commit, working tree clean.
						</div>
					{:else}
						<div class="flex items-center justify-between px-[0.7rem] py-2 border-b border-divider">
							<label class="flex items-center gap-2 text-[0.76rem] text-secondary cursor-pointer">
								<span
									class="w-4 h-4 rounded-[4px] border-[1.5px] grid place-items-center text-on-brand shrink-0 cursor-pointer {allSel
										? 'bg-brand border-brand'
										: 'border-divider-dark'}"
									role="checkbox"
									aria-checked={allSel}
									tabindex="0"
									onclick={toggleAll}
									onkeydown={(e) => (e.key === 'Enter' || e.key === ' ') && toggleAll()}
								>
									{#if allSel}<Check size={12} />{/if}
								</span>
								{selected.length} of {changes.length} selected
							</label>
							{#if selected.length}
								<button class="bg-transparent border-none text-red text-[0.76rem] cursor-pointer hover:underline" onclick={revertSelected}
									>Revert</button
								>
							{/if}
						</div>
						<ul class="list-none m-0 p-1 overflow-y-auto flex-1">
							{#each changes as c (c.path)}
								<li
									class="flex items-center gap-2 px-[0.45rem] py-[0.4rem] rounded-sm cursor-pointer group hover:bg-bg-raised {activeFile ===
									c.path
										? 'bg-button-bg'
										: ''}"
									role="button"
									tabindex="0"
									use:contextMenu={() => [
										c.conflicted && {
											label: 'Keep our version',
											icon: Check,
											onSelect: () => resolveConflict(c.path, 'ours'),
										},
										c.conflicted && {
											label: 'Keep their version',
											icon: Check,
											onSelect: () => resolveConflict(c.path, 'theirs'),
										},
										c.conflicted && { separator: true },
										{
											label: 'Revert',
											icon: RotateCcw,
											danger: true,
											onSelect: () => revert(c.path),
										},
									]}
									onclick={() => openWorkingDiff(c)}
									onkeydown={(e) => (e.key === 'Enter' || e.key === ' ') && openWorkingDiff(c)}
								>
									<span
										class="w-4 h-4 rounded-[4px] border-[1.5px] grid place-items-center text-on-brand shrink-0 cursor-pointer {isSel(
											c.path,
										)
											? 'bg-brand border-brand'
											: 'border-divider-dark'}"
										role="checkbox"
										aria-checked={isSel(c.path)}
										tabindex="0"
										onclick={(e) => {
											e.stopPropagation()
											toggle(c.path)
										}}
										onkeydown={(e) => {
											if (e.key === 'Enter' || e.key === ' ') {
												e.stopPropagation()
												toggle(c.path)
											}
										}}
									>
										{#if isSel(c.path)}<Check size={12} />{/if}
									</span>
									<span
										class="text-[0.6rem] font-semibold w-[4.1rem] shrink-0 text-center py-[0.08rem] rounded-sm lowercase {stCls[
											statusKind(c.status)
										]}">{STATUS_LABEL[c.status] ?? c.status}</span
									>
									<span
										class="flex-1 text-[0.8rem] text-body whitespace-nowrap overflow-hidden text-ellipsis"
										>{c.path}</span
									>
									{#if c.conflicted}
										<button
											class="bg-bg-inset border border-divider text-secondary text-[0.66rem] font-semibold px-[0.4rem] py-[0.1rem] rounded-sm cursor-pointer shrink-0 hover:text-contrast hover:border-divider-dark"
											title="Keep our version"
											onclick={(e) => {
												e.stopPropagation()
												resolveConflict(c.path, 'ours')
											}}>Ours</button
										>
										<button
											class="bg-bg-inset border border-divider text-secondary text-[0.66rem] font-semibold px-[0.4rem] py-[0.1rem] rounded-sm cursor-pointer shrink-0 hover:text-contrast hover:border-divider-dark"
											title="Keep their version"
											onclick={(e) => {
												e.stopPropagation()
												resolveConflict(c.path, 'theirs')
											}}>Theirs</button
										>
									{/if}
									<button
										class="bg-transparent border-none text-secondary cursor-pointer opacity-0 grid place-items-center p-[0.15rem] rounded-sm group-hover:opacity-100 hover:!text-red hover:bg-button-bg"
										use:tooltip={'Revert'}
										aria-label="Revert"
										onclick={(e) => {
											e.stopPropagation()
											revert(c.path)
										}}><RotateCcw size={13} /></button
									>
								</li>
							{/each}
						</ul>
					{/if}

					<div class="border-t border-divider p-[0.6rem]">
						<textarea
							bind:value={message}
							class="w-full min-h-[4.5rem] bg-bg-inset border border-divider text-contrast rounded-md px-[0.6rem] py-2 text-[0.84rem] resize-y outline-none focus:border-brand"
							placeholder="Commit message"
							spellcheck="false"
						></textarea>
						<label class="flex items-center gap-[0.4rem] text-[0.76rem] text-secondary mt-[0.4rem] cursor-pointer"
							><input type="checkbox" bind:checked={amend} /> Amend last commit</label
						>
						<div class="flex gap-2 mt-2">
							<ButtonStyled
								color="brand"
								size="small"
								disabled={store.busy || (!selected.length && !amend) || !message.trim()}
								onclick={doCommit}
							>
								<GitCommitHorizontal size={15} /> Commit
							</ButtonStyled>
							{#if store.git.hasRemote}
								<ButtonStyled
									type="outlined"
									size="small"
									disabled={store.busy || (!selected.length && !amend) || !message.trim()}
									onclick={doCommitPush}
								>
									Commit &amp; Push
								</ButtonStyled>
							{/if}
						</div>
					</div>

					<div class="border-t border-divider px-[0.6rem] py-2">
						<button
							class="flex items-center gap-[0.4rem] bg-transparent border-none text-secondary text-[0.78rem] cursor-pointer"
							onclick={() => (ignoreOpen = !ignoreOpen)}
						>
							<FileText size={13} /> .gitignore
						</button>
						{#if ignoreOpen}
							<div class="mt-2 flex flex-col gap-2 items-end">
								<textarea
									bind:value={ignoreContent}
									class="w-full min-h-[5rem] bg-bg-inset border border-divider text-body rounded-md p-2 font-mono text-[0.76rem] outline-none resize-y"
									spellcheck="false"
									oninput={() => (ignoreDirty = true)}
								></textarea>
								<ButtonStyled size="small" color="brand" disabled={!ignoreDirty} onclick={saveIgnore}>Save</ButtonStyled>
							</div>
						{/if}
					</div>
				</div>

				<div class="flex flex-col min-h-0 min-w-0">
					{#if diffLoading}
						<div class="flex-1 flex flex-col items-center justify-center gap-2 text-secondary text-[0.84rem]">Loading…</div>
					{:else if !rows.length}
						<div class="flex-1 flex flex-col items-center justify-center gap-2 text-secondary text-[0.84rem]">
							<FileText size={22} /> Select a file to see its diff.
						</div>
					{:else}
						<div class="flex-1 overflow-y-auto overflow-x-hidden font-mono text-[0.74rem] leading-[1.5]">
							{#each rows as row, i (i)}
								<div class="grid grid-cols-2 w-full">
									<div class="flex gap-2 px-[0.4rem] border-r border-divider min-w-0 {sideBg[row.l.cls]}">
										<span class="w-[2.6rem] shrink-0 text-right text-secondary opacity-70 select-none"
											>{row.l.n ?? ''}</span
										><span class="flex-1 min-w-0 whitespace-pre-wrap [overflow-wrap:anywhere]">{row.l.text}</span>
									</div>
									<div class="flex gap-2 px-[0.4rem] border-r border-divider min-w-0 {sideBg[row.r.cls]}">
										<span class="w-[2.6rem] shrink-0 text-right text-secondary opacity-70 select-none"
											>{row.r.n ?? ''}</span
										><span class="flex-1 min-w-0 whitespace-pre-wrap [overflow-wrap:anywhere]">{row.r.text}</span>
									</div>
								</div>
							{/each}
						</div>
					{/if}
				</div>
			{:else if tab === 'log'}
				<div class="flex flex-col border-r border-divider min-h-0">
					{#if !commits.length}
						<div class="flex flex-col items-center gap-[0.4rem] text-secondary text-center px-4 py-8 text-[0.84rem]">
							<History size={22} /> No commits yet.
						</div>
					{/if}
					<ul class="list-none m-0 p-1 overflow-y-auto flex-1">
						{#each commits as cm (cm.hash)}
							<li
								class="px-[0.55rem] py-[0.45rem] rounded-sm cursor-pointer hover:bg-bg-raised {activeCommit ===
								cm.hash
									? 'bg-button-bg'
									: ''}"
								role="button"
								tabindex="0"
								use:contextMenu={() => [
									{ label: 'New branch from here', icon: GitBranch, onSelect: () => branchFrom(cm.hash) },
									{ label: 'Tag here', icon: TagIcon, onSelect: () => tagAt(cm.hash) },
									{ label: 'Cherry-pick', onSelect: () => cherryPick(cm.hash) },
									{ label: 'Revert commit', icon: Undo2, onSelect: () => revertCommit(cm.hash) },
									{ separator: true },
									{ label: 'Reset (soft) to here', onSelect: () => resetTo(cm.hash, 'soft') },
									{ label: 'Reset (mixed) to here', onSelect: () => resetTo(cm.hash, 'mixed') },
									{ label: 'Reset (hard) to here', danger: true, onSelect: () => resetTo(cm.hash, 'hard') },
									{ separator: true },
									{ label: 'Copy hash', icon: Copy, onSelect: () => copyHash(cm.hash) },
								]}
								onclick={() => openCommit(cm.hash)}
								onkeydown={(e) => (e.key === 'Enter' || e.key === ' ') && openCommit(cm.hash)}
							>
								<div class="flex items-center gap-[0.4rem]">
									<div class="flex-1 text-[0.82rem] text-contrast font-[550] whitespace-nowrap overflow-hidden text-ellipsis">
										{cm.subject}
									</div>
									<Dropdown placement="bottom-end">
										{#snippet trigger()}
											<button
												class="{miniBtn} border-transparent bg-transparent"
												use:tooltip={'Actions'}
												aria-label="Actions"
												onclick={(e) => e.stopPropagation()}><MoreHorizontal size={14} /></button
											>
										{/snippet}
										{#snippet content(hide)}
											<div class="min-w-[190px] flex flex-col gap-px">
												<button class={mItem} onclick={() => branchFrom(cm.hash).then(hide)}
													><GitBranch size={12} /> New branch from here</button
												>
												<button class={mItem} onclick={() => tagAt(cm.hash).then(hide)}
													><TagIcon size={12} /> Tag here</button
												>
												<button class={mItem} onclick={() => cherryPick(cm.hash).then(hide)}>Cherry-pick</button>
												<button class={mItem} onclick={() => revertCommit(cm.hash).then(hide)}
													><Undo2 size={12} /> Revert commit</button
												>
												<div class={menuDivider}></div>
												<button class={mItem} onclick={() => resetTo(cm.hash, 'soft').then(hide)}>Reset (soft) to here</button>
												<button class={mItem} onclick={() => resetTo(cm.hash, 'mixed').then(hide)}>Reset (mixed) to here</button>
												<button
													class="{mItem} hover:!text-red"
													onclick={() => resetTo(cm.hash, 'hard').then(hide)}>Reset (hard) to here</button
												>
												<div class={menuDivider}></div>
												<button
													class={mItem}
													onclick={() => {
														copyHash(cm.hash)
														hide()
													}}><Copy size={12} /> Copy hash</button
												>
											</div>
										{/snippet}
									</Dropdown>
								</div>
								<div class="text-[0.7rem] text-secondary mt-[0.15rem]">
									<span class="font-mono">{cm.short}</span> · {cm.author} · {cm.relative}
								</div>
							</li>
						{/each}
					</ul>
				</div>

				<div class="flex flex-col min-h-0 min-w-0">
					{#if !activeCommit}
						<div class="flex-1 flex flex-col items-center justify-center gap-2 text-secondary text-[0.84rem]">
							<History size={22} /> Select a commit.
						</div>
					{:else}
						{#if packDiff}
							<div class="px-[0.6rem] py-[0.5rem] border-b border-divider max-h-[45%] overflow-y-auto">
								<SemanticDiff diff={packDiff} />
							</div>
						{/if}
						<div class="flex flex-wrap gap-[0.3rem] p-[0.6rem] border-b border-divider max-h-[30%] overflow-y-auto">
							{#each commitFiles as f (f.path)}
								<button
									class="inline-flex items-center gap-[0.35rem] bg-bg-inset border border-divider text-body text-[0.74rem] px-[0.5rem] py-[0.2rem] rounded-sm cursor-pointer max-w-full {activeFile ===
									f.path
										? '!border-brand text-contrast'
										: ''}"
									onclick={() => openCommitFile(f.path)}
								>
									<span
										class="text-[0.6rem] font-semibold w-[1.2rem] shrink-0 text-center py-[0.08rem] rounded-sm lowercase {stCls[
											statusKind(f.status)
										]}">{f.status}</span
									>
									{f.path}
								</button>
							{/each}
						</div>
						{#if diffLoading}
							<div class="flex-1 flex flex-col items-center justify-center gap-2 text-secondary text-[0.84rem]">Loading…</div>
						{:else if !rows.length}
							<div class="flex-1 flex flex-col items-center justify-center gap-2 text-secondary text-[0.84rem]">Select a file.</div>
						{:else}
							<div class="flex-1 overflow-y-auto overflow-x-hidden font-mono text-[0.74rem] leading-[1.5]">
								{#each rows as row, i (i)}
									<div class="grid grid-cols-2 w-full">
										<div class="flex gap-2 px-[0.4rem] border-r border-divider min-w-0 {sideBg[row.l.cls]}">
											<span class="w-[2.6rem] shrink-0 text-right text-secondary opacity-70 select-none"
												>{row.l.n ?? ''}</span
											><span class="flex-1 min-w-0 whitespace-pre-wrap [overflow-wrap:anywhere]">{row.l.text}</span>
										</div>
										<div class="flex gap-2 px-[0.4rem] border-r border-divider min-w-0 {sideBg[row.r.cls]}">
											<span class="w-[2.6rem] shrink-0 text-right text-secondary opacity-70 select-none"
												>{row.r.n ?? ''}</span
											><span class="flex-1 min-w-0 whitespace-pre-wrap [overflow-wrap:anywhere]">{row.r.text}</span>
										</div>
									</div>
								{/each}
							</div>
						{/if}
					{/if}
				</div>
			{:else}
				<ChangelogTab {commits} {tags} onsaved={() => reload(true)} />
			{/if}
		</div>
	{/if}

	{#if compareBranch}
		<Modal
			title={`Compare: ${branches.current ?? 'current'} → ${compareBranch}`}
			onclose={() => (compareBranch = null)}
		>
			{#if compareLoading}
				<div class="text-secondary px-[0.2rem] py-4">Comparing…</div>
			{:else if compareDiff}
				<SemanticDiff diff={compareDiff} />
				<div class="flex justify-end gap-2 mt-[0.8rem] pt-[0.7rem] border-t border-divider">
					<ButtonStyled type="transparent" onclick={() => (compareBranch = null)}>Close</ButtonStyled>
					<ButtonStyled color="brand" disabled={store.busy} onclick={mergeFromCompare}>
						<GitMerge size={15} /> Merge into {branches.current}
					</ButtonStyled>
				</div>
			{/if}
		</Modal>
	{/if}
</div>

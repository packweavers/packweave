<script lang="ts">
	import {
		deleteBranch,
		deleteRemoteBranch,
		mergeBranch,
		newBranch,
		openBranch,
		renameBranch,
		switchBranch,
	} from '@/lib/flows/branches'
	import { withGit } from '@/lib/flows/git'
	import { linkInstance } from '@/lib/flows/instance'
	import { git } from '@/lib/state/git.svelte'
	import { selection } from '@/lib/state/selection.svelte'
	import { tabs, type Doc } from '@/lib/state/tabs.svelte'
	import { tasks } from '@/lib/state/tasks.svelte'
	import { ws } from '@/lib/state/workspace.svelte'
	import { compare, pushBranch } from '@/lib/tauri/git'
	import type { Compare } from '@/lib/tauri/types'
	import { plural } from '@/lib/util/format'
	import { instanceFolderName, shortenHome } from '@/lib/util/path'
	import Empty from '@/ui/Empty.svelte'
	import { pending } from '@/ui/pending'

	interface Props {
		doc: Doc
	}

	let { doc }: Props = $props()

	const name = $derived(selection.of(doc.id)[0] ?? null)
	const branch = $derived((git.branches?.list ?? []).find((b) => b.name === name))
	const remoteName = $derived(name && branch?.remote ? name.slice(0, name.indexOf('/')) : null)
	const tree = $derived(name ? git.worktreeFor(name) : undefined)
	const open = $derived(tree ? git.windowFor(tree.path) : undefined)
	const current = $derived(name === git.branch)

	let diff = $state<Compare | null>(null)

	$effect(() => {
		const dir = ws.dir
		const base = git.branch
		const target = name
		diff = null
		if (!dir || !target || !base || target === base) return
		void compare(dir, base, target)
			.then((c) => {
				if (name === target) diff = c
			})
			.catch((error) => tasks.fail('Comparing branches', error))
	})
</script>

{#if branch && name}
	<div class="inspector-head">
		<div class="heading">
			<span class="strong mono">{name}</span>
			<span class="small">
				{#if current}this window{:else if diff}{plural(diff.commits.length, 'commit', 'commits')} ahead
					of {git.branch}{/if}
			</span>
		</div>
	</div>
	<div class="inspector-body">
		<section class="inspector-section">
			{#if remoteName}
				<div class="kv">
					<span class="k">Remote</span><span class="v">only on {remoteName}</span>
				</div>
			{/if}
			<div class="kv">
				<span class="k">Window</span><span class="v"
					>{current ? 'this window' : open ? 'open' : 'closed'}</span
				>
			</div>
			<div class="kv">
				<span class="k">Folder</span><span class="v mono small"
					>{tree ? shortenHome(tree.path) : 'no folder yet'}</span
				>
			</div>
			{#if !remoteName}
				<div class="kv">
					<span class="k">Tracks</span><span class="v mono small"
						>{branch.upstream
							? `${branch.upstream} ↑${branch.ahead} ↓${branch.behind}`
							: 'local only'}</span
					>
				</div>
			{/if}
			<div class="kv">
				<span class="k">Instance</span><span class="v"
					>{tree?.instance ? instanceFolderName(tree.instance) : 'none'}</span
				>
			</div>
		</section>
		<section class="inspector-section">
			{#if !current}
				<button class="item-row" onclick={() => tabs.open('compare', `${git.branch}..${name}`)}
					>Compare with {git.branch}</button
				>
				<button class="item-row" onclick={pending(() => mergeBranch(name))}
					>Merge into {git.branch}…</button
				>
			{/if}
			<button class="item-row" onclick={pending(() => newBranch(name))}
				>New branch from here…</button
			>
			{#if git.remoteName && !remoteName}
				<button
					class="item-row"
					onclick={pending(() =>
						withGit(async () => {
							if (ws.dir) await pushBranch(ws.dir, git.remoteName!, name, !branch.upstream)
							await git.refreshRefs()
						}),
					)}
				>
					Push to {git.remoteName}
				</button>
			{/if}
			{#if current}
				<button class="item-row" onclick={pending(() => linkInstance())}
					>Link a different instance…</button
				>
			{/if}
			{#if remoteName}
				<button class="item-row removed" onclick={pending(() => deleteRemoteBranch(name))}
					>Delete on {remoteName}…</button
				>
			{:else}
				<button class="item-row" onclick={pending(() => renameBranch(name))}>Rename…</button>
				<button
					class="item-row removed"
					disabled={current}
					onclick={pending(() => deleteBranch(name))}>Delete branch…</button
				>
			{/if}
		</section>
	</div>
	<div class="bar footer-bar">
		{#if !current}
			{#if !open}
				<button class="btn primary" onclick={pending(() => switchBranch(name))}>Switch to it</button
				>
			{/if}
			<button class="btn" onclick={pending(() => openBranch(name))}
				>{open ? 'Go to window' : 'Open in new window'}</button
			>
		{/if}
	</div>
{:else}
	<Empty title="Pick a branch" />
{/if}

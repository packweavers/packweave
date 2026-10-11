<script lang="ts">
	import { contentChange } from '@/lib/content/changes'
	import { newBranch } from '@/lib/flows/branches'
	import { openInFiles } from '@/lib/flows/files'
	import { cherryPick, createTag, resetTo, revertCommit } from '@/lib/flows/git'
	import { changeMark, changeWord, displayPath, isContentPath } from '@/lib/git/changes'
	import { git } from '@/lib/state/git.svelte'
	import { selection } from '@/lib/state/selection.svelte'
	import { tabs, type Doc } from '@/lib/state/tabs.svelte'
	import { tasks } from '@/lib/state/tasks.svelte'
	import { toasts } from '@/lib/state/toasts.svelte'
	import { ws } from '@/lib/state/workspace.svelte'
	import { commitChanges, commitInfo, packDiff } from '@/lib/tauri/git'
	import type { GitChange, GitCommit, PackDiff } from '@/lib/tauri/types'
	import Empty from '@/ui/Empty.svelte'
	import { pending } from '@/ui/pending'

	interface Props {
		doc: Doc
	}

	let { doc }: Props = $props()

	const hash = $derived(selection.of(doc.id)[0] ?? null)

	let commit = $state<GitCommit | null>(null)
	let files = $state<GitChange[]>([])
	let pack = $state<PackDiff | null>(null)

	$effect(() => {
		const dir = ws.dir
		const current = hash
		commit = null
		files = []
		pack = null
		if (!dir || !current) return
		void commitInfo(dir, current)
			.then((info) => {
				if (hash === current) commit = info
			})
			.catch(tasks.or('Reading the commit', undefined))
		void commitChanges(dir, current)
			.then((list) => {
				if (hash === current) files = list.filter((c) => !isContentPath(c.path))
			})
			.catch(tasks.or("Listing the commit's files", undefined))
		void packDiff(dir, `${current}^`, current)
			.then((diff) => {
				if (hash === current) pack = diff
			})
			.catch(tasks.or("Reading the commit's content changes", undefined))
	})

	function copyHash(): void {
		if (!hash) return
		navigator.clipboard
			.writeText(hash)
			.then(() => toasts.show('Copied hash'))
			.catch((error) => toasts.error(error))
	}
</script>

{#if hash}
	<div class="inspector-head">
		<div class="heading">
			<span class="strong">{commit?.subject ?? '…'}</span>
			{#if commit}
				<span class="small"
					><span class="mono">{commit.short}</span> · {commit.author} · {commit.relative}</span
				>
			{/if}
		</div>
	</div>
	<div class="inspector-body">
		<section class="inspector-section">
			<div class="band">Changes</div>
			{#each pack?.items ?? [] as item (item.path)}
				{@const w = contentChange(item.kind)}
				<div class="item-row">
					<span class="mark-text {w}">{changeMark(w)}</span>
					<span class="ellipsis">{item.name}</span>
					<span class="spacer"></span>
					<span class="mono small">{item.toVersion ?? item.fromVersion ?? ''}</span>
				</div>
			{/each}
			{#each files as file (file.path)}
				{@const w = changeWord(file)}
				<button
					class="item-row"
					onclick={() => tabs.open('diff', `${hash}:${file.path}`)}
					ondblclick={() => void openInFiles(file.path, false)}
				>
					<span class="mark-text {w}">{changeMark(w)}</span>
					<span class="ellipsis mono small">{displayPath(file.path)}</span>
				</button>
			{/each}
		</section>
		<section class="inspector-section">
			<div class="band">From here</div>
			<button class="item-row" onclick={pending(() => newBranch(hash))}>New branch here…</button>
			<button class="item-row" onclick={pending(() => createTag(hash))}>Tag here…</button>
			<button class="item-row" onclick={pending(() => cherryPick(hash))}
				>Cherry-pick onto {git.branch}</button
			>
			<button class="item-row" onclick={pending(() => revertCommit(hash))}
				>Revert this commit</button
			>
			<button class="item-row" onclick={copyHash}>Copy hash</button>
			<button class="item-row removed" onclick={pending(() => resetTo(hash))}
				>Reset {git.branch} to here…</button
			>
		</section>
	</div>
	<div class="bar footer-bar">
		<button class="btn" onclick={() => tabs.open('compare', `${hash}^..${hash}`)}>Open</button>
		<button class="btn" onclick={() => tabs.open('compare', `${hash}..${git.branch ?? 'HEAD'}`)}
			>Compare</button
		>
	</div>
{:else}
	<Empty title="Pick a commit" />
{/if}

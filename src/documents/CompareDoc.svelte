<script lang="ts">
	import { contentChange } from '@/lib/content/changes'
	import { deleteBranch, deleteRemoteBranch, mergeBranch } from '@/lib/flows/branches'
	import { openInFiles } from '@/lib/flows/files'
	import { changeMark, displayPath, isContentPath, statusWord } from '@/lib/git/changes'
	import { git } from '@/lib/state/git.svelte'
	import { tabs, type Doc } from '@/lib/state/tabs.svelte'
	import { ws } from '@/lib/state/workspace.svelte'
	import { errorText } from '@/lib/tauri/call'
	import { compare } from '@/lib/tauri/git'
	import type { Compare } from '@/lib/tauri/types'
	import { ago, plural } from '@/lib/util/format'
	import Empty from '@/ui/Empty.svelte'
	import { pending } from '@/ui/pending'

	interface Props {
		doc: Doc
	}

	let { doc }: Props = $props()

	const base = $derived(doc.arg.split('..')[0])
	const other = $derived(doc.arg.split('..')[1] ?? '')
	const isBranch = $derived(git.locals.some((b) => b.name === other) && base === git.branch)
	const isRemote = $derived(git.isRemote(other) && base === git.branch)

	let result = $state<Compare | null>(null)
	let failed = $state<string | null>(null)

	$effect(() => {
		const dir = ws.dir
		void git.head
		void git.branches
		if (!dir) return
		void compare(dir, base, other)
			.then((c) => {
				result = c
				failed = null
			})
			.catch((e) => (failed = errorText(e)))
	})

	const files = $derived((result?.files ?? []).filter((f) => !isContentPath(f.path)))
</script>

<div class="bar doc-toolbar">
	<span class="doc-title mono">{other} → {base}</span>
	{#if result}
		<span>{plural(result.commits.length, 'commit', 'commits')}</span>
		{#if result.conflicts?.length === 0}
			<span class="tag added">✓ no conflicts</span>
		{:else if result.conflicts}
			<span class="tag removed">✗ {plural(result.conflicts.length, 'conflict', 'conflicts')}</span>
		{/if}
	{/if}
	<span class="spacer"></span>
	{#if isBranch || isRemote}
		<button
			class="btn danger"
			onclick={pending(() => (isRemote ? deleteRemoteBranch(other) : deleteBranch(other)))}
			>{isRemote ? `Delete on ${other.slice(0, other.indexOf('/'))}…` : 'Delete…'}</button
		>
		<button
			class="btn primary"
			disabled={!result || result.commits.length === 0}
			onclick={pending(() => mergeBranch(other))}>Merge…</button
		>
	{/if}
</div>

{#if failed}
	<Empty title="Can't compare these" detail={failed} />
{:else if result}
	<div class="scroll">
		{#if result.pack && result.pack.items.length > 0}
			<div class="band bar">Content <span class="count-text">{result.pack.items.length}</span></div>
			{#each result.pack.items as item (item.path)}
				{@const w = contentChange(item.kind)}
				<div class="trow">
					<span class="mark {w}"></span>
					<span class="mark-text {w}">{changeMark(w)}</span>
					<span class="name">{item.name}</span>
					{#if item.dependency}<span class="tag">dependency</span>{/if}
					<span class="mono small">
						{#if item.kind === 'updated'}{item.fromVersion} → {item.toVersion}{:else}{item.toVersion ??
								item.fromVersion ??
								''}{/if}
					</span>
				</div>
			{/each}
		{/if}
		{#if files.length > 0}
			<div class="band bar">Files <span class="count-text">{files.length}</span></div>
			{#each files as file (file.path)}
				{@const w = statusWord(file.status)}
				<div
					class="trow"
					role="row"
					tabindex="-1"
					ondblclick={() => void openInFiles(file.path, false)}
				>
					<span class="mark-text {w}">{changeMark(w)}</span>
					<span class="name mono small">{displayPath(file.path)}</span>
					<span class="spacer"></span>
					<span class="mono small"
						>{file.added ? `+${file.added}` : ''} {file.removed ? `−${file.removed}` : ''}</span
					>
				</div>
			{/each}
		{/if}
		<div class="band bar">Commits <span class="count-text">{result.commits.length}</span></div>
		{#each result.commits as commit (commit.hash)}
			<button class="trow" onclick={() => tabs.open('history', commit.hash)}>
				<span class="mono small">{commit.short}</span>
				<span class="name">{commit.subject}</span>
				<span class="spacer"></span>
				<span class="small">{ago(commit.time * 1000, 'short')}</span>
			</button>
		{:else}
			<div class="trow">Nothing new on {other}</div>
		{/each}
	</div>
{:else}
	<div class="loading"><span class="spinner"></span></div>
{/if}

<style>
	.trow {
		width: 100%;
		text-align: left;
	}

	.loading {
		flex: 1;
		display: grid;
		place-items: center;
	}
</style>

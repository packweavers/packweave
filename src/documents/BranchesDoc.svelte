<script lang="ts">
	import { onMount } from 'svelte'

	import { newBranch, openBranch } from '@/lib/flows/branches'
	import { fetch, pull } from '@/lib/flows/git'
	import { branchMenu } from '@/lib/menus'
	import { Table } from '@/lib/state/columns.svelte'
	import { git } from '@/lib/state/git.svelte'
	import { menus } from '@/lib/state/menus.svelte'
	import { selection } from '@/lib/state/selection.svelte'
	import type { Doc } from '@/lib/state/tabs.svelte'
	import { instanceFolderName, shortenHome } from '@/lib/util/path'
	import ColumnHead from '@/ui/ColumnHead.svelte'
	import { pending } from '@/ui/pending'

	interface Props {
		doc: Doc
	}

	let { doc }: Props = $props()

	const table = new Table('branches', [
		{ id: 'branch', label: 'Branch', width: 180, min: 90 },
		{ id: 'window', label: 'Window', min: 160 },
		{ id: 'remote', label: 'Remote', width: 90, min: 64, hide: 1 },
		{ id: 'instance', label: 'Instance', width: 140, min: 70, hide: 2 },
		{ id: 'last', label: 'Last', width: 90, min: 56, hide: 3 },
	])
	const selected = $derived(selection.of(doc.id)[0] ?? null)

	onMount(() => {
		void git.refreshRefs()
	})

	function instanceName(dir: string | null): string {
		return dir ? instanceFolderName(dir) : 'none'
	}
</script>

<div class="bar doc-toolbar">
	<span class="doc-title">Branches</span>
	<span class="count-text">{git.locals.length}</span>
	<span class="spacer"></span>
	{#if git.remoteName}
		<button
			class="btn"
			disabled={git.busy !== null}
			aria-busy={git.busy === 'fetch'}
			onclick={() => void fetch()}>Fetch</button
		>
		<button
			class="btn"
			disabled={git.busy !== null}
			aria-busy={git.busy === 'pull'}
			onclick={() => void pull()}>Pull</button
		>
	{/if}
	<button class="btn primary" onclick={pending(newBranch)}>New branch…</button>
</div>
<div class="scroll">
	<ColumnHead {table} />
	{#each git.locals as branch (branch.name)}
		{@const tree = git.worktreeFor(branch.name)}
		{@const open = tree ? git.windowFor(tree.path) : undefined}
		<div
			class="trow"
			style={table.style}
			class:selected={selected === branch.name}
			role="option"
			aria-selected={selected === branch.name}
			tabindex="-1"
			onclick={() => selection.set(doc.id, [branch.name])}
			ondblclick={() => void openBranch(branch.name)}
			oncontextmenu={(e) => menus.at(e, branchMenu(branch.name))}
		>
			<span class="mono ellipsis">{branch.name}</span>
			<span class="c-window">
				{#if tree}
					<span class="mono small ellipsis">{shortenHome(tree.path)}</span>
					<span class="tag">{branch.current ? 'this window' : open ? 'open' : 'closed'}</span>
				{:else}
					<span class="small">no folder yet</span>
				{/if}
			</span>
			{#if table.has('remote')}
				<span class="mono small ellipsis">
					{#if branch.upstream}↑{branch.ahead} ↓{branch.behind}{:else}local only{/if}
				</span>
			{/if}
			{#if table.has('instance')}
				<span class="ellipsis">{instanceName(tree?.instance ?? null)}</span>
			{/if}
			{#if table.has('last')}
				<span class="small ellipsis">{branch.last?.replace(/ ago$/, '') ?? ''}</span>
			{/if}
		</div>
	{/each}
	{#if git.remoteOnly.length > 0}
		<div class="band bar">On {git.remoteName ?? 'origin'} only</div>
		{#each git.remoteOnly as branch (branch.name)}
			<div
				class="trow"
				style={table.style}
				class:selected={selected === branch.name}
				role="option"
				aria-selected={selected === branch.name}
				tabindex="-1"
				onclick={() => selection.set(doc.id, [branch.name])}
				ondblclick={() => void openBranch(branch.name)}
				oncontextmenu={(e) => menus.at(e, branchMenu(branch.name))}
			>
				<span class="mono ellipsis">{branch.name}</span>
				<span class="c-window"><span class="small">no folder yet</span></span>
				{#if table.has('remote')}<span class="mono small ellipsis">remote only</span>{/if}
				{#if table.has('instance')}<span class="ellipsis">none</span>{/if}
				{#if table.has('last')}
					<span class="small ellipsis">{branch.last?.replace(/ ago$/, '') ?? ''}</span>
				{/if}
			</div>
		{/each}
	{/if}
</div>

<style>
	.c-window {
		display: flex;
		align-items: center;
		gap: 8px;
		min-width: 0;
		overflow: hidden;
	}
</style>

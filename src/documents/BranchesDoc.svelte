<script lang="ts">
	import { onMount } from 'svelte'

	import { newBranch, openBranch } from '@/lib/flows/branches'
	import { fetch, pull } from '@/lib/flows/git'
	import { branchMenu } from '@/lib/menus'
	import { git } from '@/lib/state/git.svelte'
	import { menus } from '@/lib/state/menus.svelte'
	import { selection } from '@/lib/state/selection.svelte'
	import type { Doc } from '@/lib/state/tabs.svelte'
	import { instanceFolderName, shortenHome } from '@/lib/util/path'
	import { pending } from '@/ui/pending'

	interface Props {
		doc: Doc
	}

	let { doc }: Props = $props()

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
<div class="thead">
	<span class="c-branch">Branch</span>
	<span class="c-window">Window</span>
	<span class="c-remote">Remote</span>
	<span class="c-instance">Instance</span>
	<span class="c-last">Last</span>
</div>
<div class="scroll">
	{#each git.locals as branch (branch.name)}
		{@const tree = git.worktreeFor(branch.name)}
		{@const open = tree ? git.windowFor(tree.path) : undefined}
		<div
			class="trow"
			class:selected={selected === branch.name}
			role="option"
			aria-selected={selected === branch.name}
			tabindex="-1"
			onclick={() => selection.set(doc.id, [branch.name])}
			ondblclick={() => void openBranch(branch.name)}
			oncontextmenu={(e) => menus.at(e, branchMenu(branch.name))}
		>
			<span class="c-branch mono ellipsis">{branch.name}</span>
			<span class="c-window ellipsis">
				{#if tree}
					<span class="mono small ellipsis">{shortenHome(tree.path)}</span>
					<span class="tag">{branch.current ? 'this window' : open ? 'open' : 'closed'}</span>
				{:else}
					<span class="small">no folder yet</span>
				{/if}
			</span>
			<span class="c-remote mono small">
				{#if branch.upstream}↑{branch.ahead} ↓{branch.behind}{:else}local only{/if}
			</span>
			<span class="c-instance ellipsis">{instanceName(tree?.instance ?? null)}</span>
			<span class="c-last small">{branch.last?.replace(/ ago$/, '') ?? ''}</span>
		</div>
	{/each}
	{#if git.remoteOnly.length > 0}
		<div class="band bar">On {git.remoteName ?? 'origin'} only</div>
		{#each git.remoteOnly as branch (branch.name)}
			<div
				class="trow"
				class:selected={selected === branch.name}
				role="option"
				aria-selected={selected === branch.name}
				tabindex="-1"
				onclick={() => selection.set(doc.id, [branch.name])}
				ondblclick={() => void openBranch(branch.name)}
				oncontextmenu={(e) => menus.at(e, branchMenu(branch.name))}
			>
				<span class="c-branch mono ellipsis">{branch.name}</span>
				<span class="c-window ellipsis"><span class="small">no folder yet</span></span>
				<span class="c-remote mono small">remote only</span>
				<span class="c-instance ellipsis">none</span>
				<span class="c-last small">{branch.last?.replace(/ ago$/, '') ?? ''}</span>
			</div>
		{/each}
	{/if}
</div>

<style>
	.c-branch {
		flex: 0 0 180px;
	}

	.c-window {
		flex: 1 1 260px;
		display: flex;
		align-items: center;
		gap: 8px;
		min-width: 0;
	}

	.c-remote {
		flex: 0 0 90px;
	}

	.c-instance {
		flex: 0 0 140px;
	}

	.c-last {
		flex: 0 0 90px;
	}
</style>

<script lang="ts">
	import { GitBranch } from '@lucide/svelte'
	import { onMount } from 'svelte'

	import { newBranch, openBranch, switchBranch } from '@/lib/flows/branches'
	import { keysFor } from '@/lib/state/commands.svelte'
	import { git } from '@/lib/state/git.svelte'
	import { formatCombo } from '@/lib/state/keymap'
	import { fuzzyScore } from '@/lib/util/text'
	import { focusOnMount } from '@/ui/focus'
	import Icon from '@/ui/Icon.svelte'

	interface Props {
		onclose: () => void
	}

	let { onclose }: Props = $props()

	let query = $state('')
	let cursor = $state(0)

	const shown = $derived(
		[...git.locals, ...git.remoteOnly]
			.map((b) => ({ branch: b, score: fuzzyScore(query, b.name) }))
			.filter((x) => x.score !== null)
			.sort((a, b) => (b.score ?? 0) - (a.score ?? 0))
			.map((x) => x.branch),
	)

	onMount(() => {
		void git.refreshRefs()
	})

	function where(name: string): string {
		if (name === git.branch) return 'this window'
		if (git.isRemote(name)) return 'remote'
		const tree = git.worktreeFor(name)
		if (tree && git.windowFor(tree.path)) return 'open'
		return ''
	}

	function choose(name: string, newWindow: boolean): void {
		onclose()
		if (name === git.branch) return
		if (newWindow || where(name) === 'open') {
			void openBranch(name)
		} else {
			void switchBranch(name)
		}
	}

	function onkeydown(event: KeyboardEvent): void {
		if (event.key === 'ArrowDown') {
			event.preventDefault()
			cursor = Math.min(cursor + 1, shown.length - 1)
		} else if (event.key === 'ArrowUp') {
			event.preventDefault()
			cursor = Math.max(cursor - 1, 0)
		} else if (event.key === 'Enter') {
			event.preventDefault()
			const branch = shown[cursor]
			if (branch) choose(branch.name, event.altKey)
		}
	}
</script>

<div class="picker">
	<input
		class="filter"
		placeholder="Branch"
		bind:value={query}
		{onkeydown}
		use:focusOnMount
		spellcheck="false"
	/>
	<div class="list">
		{#each shown as branch, i (branch.name)}
			<button
				class="menu-item"
				class:active={i === cursor}
				onmouseenter={() => (cursor = i)}
				onclick={(e) => choose(branch.name, e.altKey)}
			>
				<Icon icon={GitBranch} size={14} />
				<span class="ellipsis">{branch.name}</span>
				<span class="keys">{where(branch.name)}</span>
			</button>
		{/each}
	</div>
	<div class="menu-sep"></div>
	<button class="menu-item" onclick={() => (onclose(), void newBranch())}>
		New branch…
		<span class="keys">{keysFor('branch.new') ?? ''}</span>
	</button>
	<div class="hint small">
		{formatCombo('Enter')} switch · {formatCombo('Alt+Enter')} new window
	</div>
</div>

<style>
	.picker {
		display: flex;
		flex-direction: column;
		padding-bottom: 4px;
	}

	.filter {
		height: var(--bar);
		padding: 0 12px;
		background: var(--band);
		box-shadow: inset 0 -1px 0 var(--field-line);
	}

	.filter:focus-visible {
		outline: none;
		box-shadow: inset 0 -2px 0 var(--focus);
	}

	.list {
		max-height: 320px;
		overflow: auto;
		padding: 4px 0;
	}

	.hint {
		padding: 4px 12px 2px;
	}
</style>

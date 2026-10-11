<script lang="ts">
	import { ChevronDown, ChevronRight } from '@lucide/svelte'
	import type { Snippet } from 'svelte'

	import Icon from '@/ui/Icon.svelte'

	interface Props {
		title: string
		count?: number | null
		open?: boolean
		actions?: Snippet
		children: Snippet
	}

	let { title, count = null, open = $bindable(true), actions, children }: Props = $props()
</script>

<section class="side-section">
	<div class="head bar">
		<button class="toggle" onclick={() => (open = !open)} aria-expanded={open}>
			<Icon icon={open ? ChevronDown : ChevronRight} size={12} />
			<span>{title}</span>
			{#if count !== null}<span class="count-text">{count}</span>{/if}
		</button>
		{#if actions}{@render actions()}{/if}
	</div>
	{#if open}
		<div class="body">{@render children()}</div>
	{/if}
</section>

<style>
	.side-section {
		display: flex;
		flex-direction: column;
	}

	.head {
		height: var(--row);
		padding-left: 0;
		background: var(--band);
		font-size: 12.5px;
		font-weight: 600;
	}

	.toggle {
		flex: 1;
		align-self: stretch;
		display: flex;
		align-items: center;
		gap: 6px;
		padding-left: 10px;
		min-width: 0;
	}

	.toggle .count-text {
		margin-left: auto;
		padding-right: 4px;
	}

	.head :global(.icon-btn) {
		height: auto;
		align-self: stretch;
	}

	.body {
		display: flex;
		flex-direction: column;
		padding: 2px 0 6px;
	}
</style>

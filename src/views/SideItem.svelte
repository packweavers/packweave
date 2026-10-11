<script lang="ts">
	import type { Component, Snippet } from 'svelte'

	import Icon from '@/ui/Icon.svelte'

	interface Props {
		label: string
		id?: string
		icon?: Component
		depth?: number
		active?: boolean
		count?: number | null
		mono?: boolean
		trailing?: Snippet
		lead?: Snippet
		onclick?: (event: MouseEvent) => void
		ondblclick?: (event: MouseEvent) => void
		oncontextmenu?: (event: MouseEvent) => void
	}

	let {
		label,
		id,
		icon,
		depth = 0,
		active = false,
		count = null,
		mono = false,
		trailing,
		lead,
		onclick,
		ondblclick,
		oncontextmenu,
	}: Props = $props()
</script>

<div
	{id}
	class="side-item"
	class:active
	style:padding-left="{16 + depth * 14}px"
	role="treeitem"
	aria-selected={active}
	tabindex="-1"
	{onclick}
	{ondblclick}
	{oncontextmenu}
>
	{#if lead}{@render lead()}{/if}
	{#if icon}<Icon {icon} size={14} />{/if}
	<span class="label ellipsis" class:mono>{label}</span>
	{#if count !== null}<span class="count-text">{count}</span>{/if}
	{#if trailing}{@render trailing()}{/if}
</div>

<style>
	.side-item {
		position: relative;
		flex: none;
		height: var(--row);
		display: flex;
		align-items: center;
		gap: 8px;
		padding-right: 14px;
		white-space: nowrap;
		min-width: 0;
	}

	.side-item:hover {
		background: var(--hover);
	}

	.side-item.active {
		background: var(--selected);
	}

	.label {
		flex: 1;
	}

	.label.mono {
		font-size: 12px;
	}
</style>

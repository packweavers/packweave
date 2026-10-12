<script lang="ts">
	import type { Table } from '@/lib/state/columns.svelte'

	import Splitter from './Splitter.svelte'

	interface Props {
		table: Table
		sticky?: boolean
	}

	let { table, sticky = true }: Props = $props()

	const flex = $derived(table.shown.findIndex((c) => c.width === undefined))
</script>

<div
	class="column-head"
	class:sticky
	style="{table.style}; --gap: {table.gap}px"
	{@attach table.measure}
>
	{#each table.shown as column, i (column.id)}
		<span class="column">
			<span class="ellipsis">{column.label}</span>
			{#if column.width !== undefined}
				<span class="handle" class:after={i < flex}>
					<Splitter
						axis="x"
						value={table.width(column)}
						min={column.min}
						max={column.max ?? 640}
						invert={i > flex}
						onchange={(width) => table.resize(column, width)}
						ondone={() => table.persist()}
						onreset={() => table.reset(column)}
					/>
				</span>
			{/if}
		</span>
	{/each}
</div>

<style>
	.column-head {
		flex: none;
		align-items: center;
		height: var(--row);
		background: var(--band);
		border-bottom: 1px solid var(--line);
		font-size: 12.5px;
		font-weight: 600;
		white-space: nowrap;
	}

	.column-head.sticky {
		position: sticky;
		top: 0;
		z-index: 2;
	}

	.column {
		position: relative;
		display: flex;
		align-items: center;
		align-self: stretch;
		min-width: 0;
	}

	.handle {
		position: absolute;
		top: 0;
		bottom: 0;
		left: calc(var(--gap) / -2);
		width: 0;
	}

	.handle.after {
		left: auto;
		right: calc(var(--gap) / -2);
	}
</style>

<script lang="ts">
	import { ChevronDown, ChevronRight } from '@lucide/svelte'

	import { Table } from '@/lib/state/columns.svelte'
	import type { NbtNode } from '@/lib/tauri/types'

	import ColumnHead from './ColumnHead.svelte'
	import Icon from './Icon.svelte'

	interface Props {
		root: NbtNode
	}

	let { root }: Props = $props()

	interface Row {
		node: NbtNode
		path: string
		depth: number
	}

	const table = new Table('nbt', [
		{ id: 'name', label: 'Name', width: 260, min: 120 },
		{ id: 'type', label: 'Type', width: 140, min: 64, hide: 1 },
		{ id: 'value', label: 'Value', min: 120 },
	])
	let collapsed = $state<Record<string, boolean>>({})

	const rows = $derived.by(() => {
		const out: Row[] = []
		const walk = (node: NbtNode, path: string, depth: number) => {
			out.push({ node, path, depth })
			if (collapsed[path]) return
			node.children.forEach((child, i) => walk(child, `${path}.${i}`, depth + 1))
		}
		walk(root, 'root', 0)
		return out
	})

	function summary(node: NbtNode): string {
		if (node.children.length > 0) return `${node.tag} · ${node.children.length}`
		return node.tag
	}
</script>

<div class="nbt" role="tree">
	<ColumnHead {table} />
	{#each rows as row (row.path)}
		<div class="trow" role="treeitem" aria-selected="false" style={table.style}>
			<span class="name" style:padding-left="{row.depth * 16}px">
				{#if row.node.children.length > 0}
					<button
						class="twisty"
						aria-label={collapsed[row.path] ? 'Expand' : 'Collapse'}
						onclick={() => (collapsed[row.path] = !collapsed[row.path])}
					>
						<Icon icon={collapsed[row.path] ? ChevronRight : ChevronDown} size={12} />
					</button>
				{:else}
					<span class="twisty"></span>
				{/if}
				<span class="ellipsis">{row.depth === 0 ? 'root' : row.node.name}</span>
			</span>
			{#if table.has('type')}<span class="mono ellipsis">{summary(row.node)}</span>{/if}
			<span class="mono ellipsis">{row.node.value ?? ''}</span>
		</div>
	{/each}
</div>

<style>
	.nbt {
		flex: 1;
		min-height: 0;
		overflow: auto;
		user-select: text;
		-webkit-user-select: text;
	}

	.name {
		display: flex;
		align-items: center;
		gap: 4px;
		min-width: 0;
	}

	.twisty {
		width: 14px;
		display: inline-grid;
		place-items: center;
		flex: none;
	}
</style>

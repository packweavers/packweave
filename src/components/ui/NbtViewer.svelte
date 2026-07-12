<script lang="ts">
	import { ChevronRight } from '@lucide/svelte'
	import type { NbtNode } from '../../types'

	let {
		root,
		query = '',
		active = 0,
		matchCount = $bindable(0),
	}: { root: NbtNode; query?: string; active?: number; matchCount?: number } = $props()

	let collapsed = $state(new Set<string>())
	let container = $state<HTMLElement>()

	function toggle(path: string) {
		if (collapsed.has(path)) collapsed.delete(path)
		else collapsed.add(path)
		collapsed = new Set(collapsed)
	}

	const matches = $derived.by(() => {
		const q = query.trim().toLowerCase()
		if (!q) return [] as string[]
		const out: string[] = []
		const walk = (n: NbtNode, path: string) => {
			if (n.name.toLowerCase().includes(q) || (n.value ?? '').toLowerCase().includes(q)) {
				out.push(path)
			}
			n.children.forEach((c, i) => walk(c, path + '.' + i))
		}
		walk(root, 'r')
		return out
	})
	$effect(() => {
		matchCount = matches.length
	})

	const matchSet = $derived(new Set(matches))
	const activePath = $derived(matches[active] ?? null)
	const forced = $derived.by(() => {
		const s = new Set<string>()
		if (!activePath) return s
		const parts = activePath.split('.')
		for (let i = 1; i < parts.length; i++) s.add(parts.slice(0, i).join('.'))
		return s
	})
	const isOpen = (path: string) => forced.has(path) || !collapsed.has(path)

	$effect(() => {
		const p = activePath
		if (!p || !container) return
		requestAnimationFrame(() => {
			container?.querySelector(`[data-path="${p}"]`)?.scrollIntoView({ block: 'center' })
		})
	})

	const tagColor: Record<string, string> = {
		compound: 'text-purple',
		list: 'text-blue',
		string: 'text-green',
		byteArray: 'text-blue',
		intArray: 'text-blue',
		longArray: 'text-blue',
	}
	const abbr: Record<string, string> = {
		compound: '{}',
		list: '[]',
		byteArray: 'B[]',
		intArray: 'I[]',
		longArray: 'L[]',
		string: 'str',
		byte: 'b',
		short: 's',
		int: 'i',
		long: 'l',
		float: 'f',
		double: 'd',
	}
</script>

{#snippet node(n: NbtNode, path: string, depth: number)}
	{@const kids = n.children.length}
	{@const open = isOpen(path)}
	{@const hit = matchSet.has(path)}
	{@const isActive = path === activePath}
	<div>
		<div
			data-path={path}
			class="flex items-baseline gap-1.5 py-[0.1rem] rounded-sm {isActive
				? 'bg-brand-highlight'
				: hit
					? 'bg-orange/15'
					: 'hover:bg-bg-raised'}"
			style="padding-left:{depth * 0.85 + 0.2}rem"
		>
			{#if kids}
				<button
					type="button"
					class="grid place-items-center w-3.5 h-3.5 shrink-0 self-center text-secondary bg-transparent border-none cursor-pointer"
					onclick={() => toggle(path)}
					aria-label={open ? 'Collapse' : 'Expand'}
				>
					<ChevronRight size={11} class={`transition-transform ${open ? 'rotate-90' : ''}`} />
				</button>
			{:else}
				<span class="w-3.5 shrink-0"></span>
			{/if}
			<span class="shrink-0 w-7 text-[0.62rem] font-bold {tagColor[n.tag] ?? 'text-orange'}"
				>{abbr[n.tag] ?? n.tag}</span
			>
			<span class="text-contrast font-medium shrink-0">{n.name === '' ? '(root)' : n.name}</span>
			{#if kids}
				<span class="text-secondary text-[0.72rem]">{kids} {kids === 1 ? 'entry' : 'entries'}</span>
			{:else if n.value != null}
				<span class="text-secondary min-w-0 [overflow-wrap:anywhere]">{n.value}</span>
			{/if}
		</div>
		{#if kids && open}
			{#each n.children as c, i (path + '.' + i)}
				{@render node(c, path + '.' + i, depth + 1)}
			{/each}
		{/if}
	</div>
{/snippet}

<div bind:this={container} class="flex-1 min-h-0 overflow-auto p-2 font-mono text-[0.78rem] leading-[1.5]">
	{@render node(root, 'r', 0)}
</div>

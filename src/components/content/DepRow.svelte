<script lang="ts">
	import { ArrowUpToLine } from '@lucide/svelte'
	import Avatar from '../ui/Avatar.svelte'
	import Badge from '../ui/Badge.svelte'
	import { tooltip } from '../../lib/tooltip'
	import { contextMenu } from '../../lib/contextmenu.svelte'
	import { store } from '../../lib/store.svelte'
	import { activeSource } from '../../types'
	import type { LockedMod } from '../../types'

	let { mod, nameOf }: { mod: LockedMod; nameOf: (id: string) => string } = $props()

	const meta = $derived(store.meta[mod.projectId])
</script>

<li
	class="group flex items-center gap-[0.7rem] px-[0.6rem] py-2 rounded-md border border-transparent opacity-90 hover:bg-bg-raised hover:border-divider"
	use:contextMenu={() => [
		{
			label: 'Make standalone',
			icon: ArrowUpToLine,
			disabled: store.busy,
			onSelect: () => store.promoteMod(mod.projectId),
		},
	]}
>
	<Avatar src={meta?.iconUrl ?? null} alt={mod.name} size={30} />
	<div class="flex-1 min-w-0">
		<div class="flex items-center gap-2">
			<span class="font-semibold text-contrast whitespace-nowrap overflow-hidden text-ellipsis"
				>{mod.name}</span
			>
			<span class="font-mono text-[0.7rem] text-secondary">{activeSource(mod)?.versionNumber}</span>
		</div>
		<div class="flex items-center gap-[0.4rem] mt-[0.2rem] text-[0.74rem] text-secondary">
			{#if meta?.author}
				<span class="inline-flex items-center gap-[0.3rem] text-body">{meta?.author}</span>
			{/if}
			<span class="opacity-50">·</span>
			<span class="text-secondary">via {mod.dependents.map(nameOf).join(', ')}</span>
		</div>
	</div>
	<button
		class="grid place-items-center w-[1.8rem] h-[1.8rem] rounded-sm border-none bg-transparent text-secondary cursor-pointer opacity-0 group-hover:opacity-100 shrink-0 hover:bg-button-bg hover:text-contrast disabled:opacity-60"
		aria-label="Make standalone"
		use:tooltip={'Make standalone, kept even if its parent is removed'}
		disabled={store.busy}
		onclick={() => store.promoteMod(mod.projectId)}
	>
		<ArrowUpToLine size={14} />
	</button>
	<Badge>dependency</Badge>
</li>

<script lang="ts">
	import { ctxMenu, closeContextMenu } from '../../lib/contextmenu.svelte'

	let menuEl = $state<HTMLElement>()
	let adj = $state({ x: 0, y: 0 })

	$effect(() => {
		if (!ctxMenu.open) return
		adj = { x: ctxMenu.x, y: ctxMenu.y }
		requestAnimationFrame(() => {
			if (!menuEl) return
			const r = menuEl.getBoundingClientRect()
			let x = ctxMenu.x
			let y = ctxMenu.y
			if (x + r.width > window.innerWidth - 8) x = window.innerWidth - r.width - 8
			if (y + r.height > window.innerHeight - 8) y = window.innerHeight - r.height - 8
			adj = { x: Math.max(8, x), y: Math.max(8, y) }
		})
	})
</script>

<svelte:window onkeydown={(e) => ctxMenu.open && e.key === 'Escape' && closeContextMenu()} />

{#if ctxMenu.open}
	<div
		class="fixed inset-0 z-[100]"
		role="presentation"
		onclick={closeContextMenu}
		oncontextmenu={(e) => {
			e.preventDefault()
			closeContextMenu()
		}}
	>
		<div
			bind:this={menuEl}
			class="absolute min-w-[200px] max-w-[300px] bg-bg-super-raised border border-divider rounded-md shadow-floating p-1.5 [backdrop-filter:saturate(180%)_blur(20px)] flex flex-col gap-px"
			style="left:{adj.x}px;top:{adj.y}px"
			role="menu"
			tabindex="-1"
		>
			{#each ctxMenu.items as it, i (i)}
				{#if it.separator}
					<div class="h-px bg-divider my-1"></div>
				{:else}
					{@const Icon = it.icon}
					<button
						type="button"
						role="menuitem"
						disabled={it.disabled}
						class="flex items-center gap-2 bg-transparent border-none text-left text-[0.8rem] px-2 py-[0.4rem] rounded-sm cursor-pointer w-full disabled:opacity-40 disabled:cursor-default {it.danger
							? 'text-red enabled:hover:bg-red/[0.12]'
							: 'text-body enabled:hover:bg-button-bg enabled:hover:text-contrast'}"
						onclick={() => {
							closeContextMenu()
							it.onSelect?.()
						}}
					>
						{#if Icon}<Icon size={14} class="shrink-0" />{/if}
						<span class="flex-1 min-w-0 whitespace-nowrap overflow-hidden text-ellipsis">{it.label}</span>
					</button>
				{/if}
			{/each}
		</div>
	</div>
{/if}

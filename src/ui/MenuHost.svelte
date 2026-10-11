<script lang="ts">
	import { keysFor } from '@/lib/state/commands.svelte'
	import { menus, type MenuEntry, type MenuItem } from '@/lib/state/menus.svelte'

	import Icon from './Icon.svelte'

	let element = $state<HTMLDivElement | null>(null)
	let cursor = $state(-1)
	let position = $state({ x: 0, y: 0 })

	const open = $derived(menus.current)
	const items = $derived((open?.entries ?? []).map((entry, index) => ({ entry, index })))

	function isItem(entry: MenuEntry): entry is MenuItem {
		return typeof entry === 'object' && 'run' in entry
	}

	$effect(() => {
		if (!open || !element) return
		cursor = -1
		const box = element.getBoundingClientRect()
		position = {
			x: Math.max(4, Math.min(open.x, window.innerWidth - box.width - 4)),
			y: open.y + box.height > window.innerHeight - 4 ? Math.max(4, open.y - box.height) : open.y,
		}
		element.focus()
	})

	function choose(item: MenuItem): void {
		if (item.disabled) return
		menus.close()
		item.run()
	}

	function move(step: number): void {
		const runnable = items.filter(({ entry }) => isItem(entry) && !entry.disabled)
		if (runnable.length === 0) return
		const at = runnable.findIndex(({ index }) => index === cursor)
		const next = runnable[(at + step + runnable.length) % runnable.length]
		cursor = next.index
	}

	function onkeydown(event: KeyboardEvent): void {
		if (event.key === 'Escape') {
			event.preventDefault()
			menus.close()
		} else if (event.key === 'ArrowDown') {
			event.preventDefault()
			move(1)
		} else if (event.key === 'ArrowUp') {
			event.preventDefault()
			move(-1)
		} else if (event.key === 'Enter') {
			event.preventDefault()
			const entry = open?.entries[cursor]
			if (entry && isItem(entry)) choose(entry)
		}
	}
</script>

<svelte:window onblur={() => menus.close()} onresize={() => menus.close()} />

{#if open}
	<div
		class="shield"
		role="presentation"
		onmousedown={() => menus.close()}
		oncontextmenu={(e) => {
			e.preventDefault()
			menus.close()
		}}
	></div>
	<div
		bind:this={element}
		class="menu"
		role="menu"
		tabindex="-1"
		style:left="{position.x}px"
		style:top="{position.y}px"
		{onkeydown}
	>
		{#each items as { entry, index } (index)}
			{#if entry === 'separator'}
				<div class="menu-sep" role="separator"></div>
			{:else if 'heading' in entry}
				<div class="menu-heading">{entry.heading}</div>
			{:else}
				{@const keys = entry.command ? keysFor(entry.command) : null}
				<button
					class="menu-item"
					class:danger={entry.danger}
					class:active={index === cursor || entry.selected}
					role="menuitem"
					disabled={entry.disabled}
					onmouseenter={() => (cursor = index)}
					onclick={() => choose(entry)}
				>
					{#if entry.icon}
						<Icon icon={entry.icon} size={14} />
					{/if}
					<span class="ellipsis">{entry.label}</span>
					{#if keys}
						<span class="keys">{keys}</span>
					{/if}
				</button>
			{/if}
		{/each}
	</div>
{/if}

<style>
	.shield {
		position: fixed;
		inset: 0;
		z-index: 59;
	}
</style>

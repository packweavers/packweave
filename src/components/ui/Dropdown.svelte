<script lang="ts">
	import { computePosition, flip, shift, offset, autoUpdate, type Placement } from '@floating-ui/dom'
	import type { Snippet } from 'svelte'

	let {
		placement = 'bottom-start',
		trigger,
		content,
	}: {
		placement?: Placement
		trigger: Snippet
		content: Snippet<[() => void]>
	} = $props()

	let open = $state(false)
	let triggerEl = $state<HTMLElement>()
	let menuEl = $state<HTMLElement>()

	function reposition() {
		if (!triggerEl || !menuEl) return
		computePosition(triggerEl, menuEl, {
			placement,
			strategy: 'fixed',
			middleware: [offset(6), flip(), shift({ padding: 8 })],
		}).then(({ x, y }) => {
			if (menuEl) {
				menuEl.style.left = `${x}px`
				menuEl.style.top = `${y}px`
			}
		})
	}

	$effect(() => {
		if (open && triggerEl && menuEl) {
			return autoUpdate(triggerEl, menuEl, reposition)
		}
	})

	const close = () => (open = false)

	function onDocClick(e: MouseEvent) {
		if (!open) return
		const t = e.target as Node
		if (triggerEl?.contains(t) || menuEl?.contains(t)) return
		open = false
	}

	function onEscape(e: KeyboardEvent) {
		if (open && e.key === 'Escape') {
			e.stopPropagation()
			open = false
		}
	}

	function portal(node: HTMLElement) {
		document.body.appendChild(node)
		return {
			destroy() {
				node.remove()
			},
		}
	}
</script>

<svelte:document onclick={onDocClick} onkeydowncapture={onEscape} />

<span
	bind:this={triggerEl}
	class="inline-flex"
	role="button"
	tabindex="0"
	onclick={() => (open = !open)}
	onkeydown={(e) => {
		if (e.key === 'Enter' || e.key === ' ') {
			e.preventDefault()
			open = !open
		}
	}}
>
	{@render trigger()}
</span>

{#if open}
	<div
		bind:this={menuEl}
		use:portal
		data-portal-root
		class="fixed z-[70] bg-bg-super-raised border border-divider rounded-md shadow-floating p-1.5"
		style="left:0;top:0"
	>
		{@render content(close)}
	</div>
{/if}

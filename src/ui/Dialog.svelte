<script lang="ts">
	import { onMount, type Snippet } from 'svelte'

	interface Props {
		title: string
		width?: 440 | 560 | 720
		onclose: () => void
		children: Snippet
		footer?: Snippet
	}

	let { title, width = 440, onclose, children, footer }: Props = $props()

	let element: HTMLDivElement

	onMount(() => {
		if (element.contains(document.activeElement)) return
		const target =
			element.querySelector<HTMLElement>(
				'.dialog-body input:not([type=checkbox]):not([type=radio]):not([disabled]), .dialog-body textarea:not([disabled])',
			) ?? element
		target.focus()
	})

	function onkeydown(event: KeyboardEvent): void {
		if (event.key === 'Escape') {
			event.preventDefault()
			event.stopPropagation()
			onclose()
			return
		}
		if (event.key !== 'Enter' || event.target !== element) return
		const button = element.querySelector<HTMLButtonElement>('[data-default]:not(:disabled)')
		if (!button) return
		event.preventDefault()
		button.click()
	}
</script>

<div class="scrim" role="presentation">
	<div
		bind:this={element}
		class="dialog"
		role="dialog"
		aria-modal="true"
		aria-label={title}
		tabindex="-1"
		style:width="{width}px"
		{onkeydown}
	>
		<div class="dialog-head">
			<span class="dialog-title">{title}</span>
			<button class="kbd" onclick={onclose} aria-label="Close">esc</button>
		</div>
		<div class="dialog-body">
			{@render children()}
		</div>
		{#if footer}
			<div class="dialog-foot">
				{@render footer()}
			</div>
		{/if}
	</div>
</div>

<script lang="ts">
	import { computePosition, flip, offset, shift, type Placement } from '@floating-ui/dom'
	import type { Snippet } from 'svelte'

	interface Props {
		anchor: HTMLElement
		placement?: Placement
		width?: number
		onclose: () => void
		children: Snippet
	}

	let { anchor, placement = 'bottom-start', width, onclose, children }: Props = $props()

	let element: HTMLDivElement
	let position = $state({ x: 0, y: 0 })

	$effect(() => {
		void computePosition(anchor, element, {
			placement,
			strategy: 'fixed',
			middleware: [offset(4), flip(), shift({ padding: 6 })],
		}).then(({ x, y }) => (position = { x, y }))
	})

	function onpointerdown(event: PointerEvent): void {
		const target = event.target as Node
		if (!element.contains(target) && !anchor.contains(target)) onclose()
	}

	function onkeydown(event: KeyboardEvent): void {
		if (event.key === 'Escape') {
			event.preventDefault()
			onclose()
		}
	}
</script>

<svelte:window {onpointerdown} {onkeydown} onblur={onclose} />

<div
	bind:this={element}
	class="popover"
	role="dialog"
	style:left="{position.x}px"
	style:top="{position.y}px"
	style:width={width ? `${width}px` : undefined}
>
	{@render children()}
</div>

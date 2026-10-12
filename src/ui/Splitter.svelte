<script lang="ts">
	interface Props {
		axis: 'x' | 'y'
		value: number
		min: number
		max: number
		invert?: boolean
		onchange?: (value: number) => void
		ondone?: () => void
		onreset?: () => void
	}

	let {
		axis,
		value = $bindable(),
		min,
		max,
		invert = false,
		onchange,
		ondone,
		onreset,
	}: Props = $props()

	let dragging = $state(false)

	function onpointerdown(event: PointerEvent): void {
		event.preventDefault()
		const handle = event.currentTarget as HTMLElement
		handle.setPointerCapture(event.pointerId)
		const start = axis === 'x' ? event.clientX : event.clientY
		const from = value
		dragging = true
		const move = (e: PointerEvent) => {
			const delta = (axis === 'x' ? e.clientX : e.clientY) - start
			value = Math.round(Math.min(max, Math.max(min, from + (invert ? -delta : delta))))
			onchange?.(value)
		}
		const up = () => {
			dragging = false
			handle.removeEventListener('pointermove', move)
			handle.removeEventListener('pointerup', up)
			ondone?.()
		}
		handle.addEventListener('pointermove', move)
		handle.addEventListener('pointerup', up)
	}
</script>

<div
	class="splitter {axis}"
	class:dragging
	role="separator"
	aria-orientation={axis === 'x' ? 'vertical' : 'horizontal'}
	{onpointerdown}
	ondblclick={onreset}
></div>

<style>
	.splitter {
		position: absolute;
		z-index: 5;
	}

	.splitter.x {
		top: 0;
		bottom: 0;
		width: 6px;
		margin-left: -3px;
		cursor: col-resize;
	}

	.splitter.y {
		left: 0;
		right: 0;
		height: 6px;
		margin-top: -3px;
		cursor: row-resize;
	}

	.splitter.dragging,
	.splitter:hover {
		background: color-mix(in srgb, var(--focus) 40%, transparent);
	}
</style>

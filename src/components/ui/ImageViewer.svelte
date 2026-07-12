<script lang="ts">
	import { ZoomIn, ZoomOut, Maximize } from '@lucide/svelte'

	let { src }: { src: string } = $props()

	let box = $state<HTMLDivElement | null>(null)
	let scale = $state(1)
	let tx = $state(0)
	let ty = $state(0)
	let dragging = false
	let lastX = 0
	let lastY = 0

	function clamp(v: number) {
		return Math.min(8, Math.max(0.1, v))
	}

	function zoomAt(factor: number, cx: number, cy: number) {
		const next = clamp(scale * factor)
		const k = next / scale
		tx = cx - (cx - tx) * k
		ty = cy - (cy - ty) * k
		scale = next
	}

	function onWheel(e: WheelEvent) {
		e.preventDefault()
		if (!box) return
		const rect = box.getBoundingClientRect()
		const cx = e.clientX - rect.left - rect.width / 2
		const cy = e.clientY - rect.top - rect.height / 2
		zoomAt(e.deltaY < 0 ? 1.12 : 1 / 1.12, cx, cy)
	}

	function zoomBtn(factor: number) {
		zoomAt(factor, 0, 0)
	}

	function reset() {
		scale = 1
		tx = 0
		ty = 0
	}

	function onMove(e: MouseEvent) {
		if (!dragging) return
		tx += e.clientX - lastX
		ty += e.clientY - lastY
		lastX = e.clientX
		lastY = e.clientY
	}

	function onUp() {
		dragging = false
		window.removeEventListener('mousemove', onMove)
		window.removeEventListener('mouseup', onUp)
	}

	function onDown(e: MouseEvent) {
		e.preventDefault()
		dragging = true
		lastX = e.clientX
		lastY = e.clientY
		window.addEventListener('mousemove', onMove)
		window.addEventListener('mouseup', onUp)
	}
</script>

<div
	bind:this={box}
	onwheel={onWheel}
	onmousedown={onDown}
	ondblclick={reset}
	class="flex-1 relative overflow-hidden grid place-items-center cursor-grab active:cursor-grabbing [background:repeating-conic-gradient(var(--color-bg)_0%_25%,var(--color-bg-raised)_0%_50%)_50%/20px_20px]"
>
	<img
		{src}
		style="transform: translate({tx}px, {ty}px) scale({scale})"
		draggable="false"
		alt=""
		class="max-w-full max-h-full object-contain [image-rendering:pixelated] select-none [will-change:transform]"
	/>
	<div
		onmousedown={(e) => e.stopPropagation()}
		ondblclick={(e) => e.stopPropagation()}
		role="toolbar"
		tabindex="-1"
		aria-label="Image controls"
		class="absolute bottom-[0.7rem] left-1/2 -translate-x-1/2 flex items-center gap-[0.15rem] bg-bg-raised border border-divider rounded-max px-[0.35rem] py-[0.2rem] shadow-floating cursor-default"
	>
		<button
			title="Zoom out"
			onclick={() => zoomBtn(1 / 1.2)}
			class="grid place-items-center bg-transparent border-none text-secondary cursor-pointer p-1 rounded-sm hover:bg-button-bg hover:text-contrast"
		>
			<ZoomOut size={15} />
		</button>
		<span class="text-[0.72rem] text-secondary min-w-[2.8rem] text-center">{Math.round(scale * 100)}%</span>
		<button
			title="Zoom in"
			onclick={() => zoomBtn(1.2)}
			class="grid place-items-center bg-transparent border-none text-secondary cursor-pointer p-1 rounded-sm hover:bg-button-bg hover:text-contrast"
		>
			<ZoomIn size={15} />
		</button>
		<button
			title="Fit"
			onclick={reset}
			class="grid place-items-center bg-transparent border-none text-secondary cursor-pointer p-1 rounded-sm hover:bg-button-bg hover:text-contrast"
		>
			<Maximize size={14} />
		</button>
	</div>
</div>

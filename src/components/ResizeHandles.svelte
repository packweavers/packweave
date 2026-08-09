<script lang="ts">
	import { onMount } from 'svelte'
	import { getCurrentWindow } from '@tauri-apps/api/window'

	type Direction =
		| 'East'
		| 'North'
		| 'NorthEast'
		| 'NorthWest'
		| 'South'
		| 'SouthEast'
		| 'SouthWest'
		| 'West'

	const win = getCurrentWindow()
	let maximized = $state(false)

	async function refresh() {
		try {
			maximized = await win.isMaximized()
		} catch {
			maximized = false
		}
	}

	onMount(() => {
		void refresh()
		const unlisten = win.onResized(() => void refresh())
		return () => {
			void unlisten.then((u) => u())
		}
	})

	function start(direction: Direction) {
		return (e: MouseEvent) => {
			if (e.button !== 0) return
			e.preventDefault()
			void win.startResizeDragging(direction)
		}
	}
</script>

{#if !maximized}
	<div
		aria-hidden="true"
		class="fixed top-0 left-2 right-2 h-[4px] z-[999] cursor-n-resize"
		onmousedown={start('North')}
	></div>
	<div
		aria-hidden="true"
		class="fixed bottom-0 left-2 right-2 h-[4px] z-[999] cursor-s-resize"
		onmousedown={start('South')}
	></div>
	<div
		aria-hidden="true"
		class="fixed top-2 bottom-2 left-0 w-[4px] z-[999] cursor-w-resize"
		onmousedown={start('West')}
	></div>
	<div
		aria-hidden="true"
		class="fixed top-2 bottom-2 right-0 w-[4px] z-[999] cursor-e-resize"
		onmousedown={start('East')}
	></div>
	<div
		aria-hidden="true"
		class="fixed top-0 left-0 w-2 h-2 z-[999] cursor-nw-resize"
		onmousedown={start('NorthWest')}
	></div>
	<div
		aria-hidden="true"
		class="fixed top-0 right-0 w-2 h-2 z-[999] cursor-ne-resize"
		onmousedown={start('NorthEast')}
	></div>
	<div
		aria-hidden="true"
		class="fixed bottom-0 left-0 w-2 h-2 z-[999] cursor-sw-resize"
		onmousedown={start('SouthWest')}
	></div>
	<div
		aria-hidden="true"
		class="fixed bottom-0 right-0 w-2 h-2 z-[999] cursor-se-resize"
		onmousedown={start('SouthEast')}
	></div>
{/if}

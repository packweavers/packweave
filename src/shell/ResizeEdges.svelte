<script lang="ts">
	import { getCurrentWindow } from '@tauri-apps/api/window'

	import { windowState } from '@/lib/state/window.svelte'

	type Direction =
		'East' | 'North' | 'NorthEast' | 'NorthWest' | 'South' | 'SouthEast' | 'SouthWest' | 'West'

	const win = getCurrentWindow()
	const edges: { direction: Direction; area: string }[] = [
		{ direction: 'North', area: 'n' },
		{ direction: 'South', area: 's' },
		{ direction: 'West', area: 'w' },
		{ direction: 'East', area: 'e' },
		{ direction: 'NorthWest', area: 'nw' },
		{ direction: 'NorthEast', area: 'ne' },
		{ direction: 'SouthWest', area: 'sw' },
		{ direction: 'SouthEast', area: 'se' },
	]

	function start(direction: Direction, event: MouseEvent): void {
		if (event.button !== 0) return
		event.preventDefault()
		void win.startResizeDragging(direction)
	}
</script>

{#if !windowState.maximized}
	{#each edges as edge (edge.area)}
		<div
			class="edge {edge.area}"
			aria-hidden="true"
			onmousedown={(e) => start(edge.direction, e)}
		></div>
	{/each}
{/if}

<style>
	.edge {
		position: fixed;
		z-index: 90;
	}

	.n,
	.s {
		left: 8px;
		right: 8px;
		height: 4px;
	}

	.n {
		top: 0;
		cursor: n-resize;
	}

	.s {
		bottom: 0;
		cursor: s-resize;
	}

	.w,
	.e {
		top: 8px;
		bottom: 8px;
		width: 4px;
	}

	.w {
		left: 0;
		cursor: w-resize;
	}

	.e {
		right: 0;
		cursor: e-resize;
	}

	.nw,
	.ne,
	.sw,
	.se {
		width: 8px;
		height: 8px;
	}

	.nw {
		top: 0;
		left: 0;
		cursor: nw-resize;
	}

	.ne {
		top: 0;
		right: 0;
		cursor: ne-resize;
	}

	.sw {
		bottom: 0;
		left: 0;
		cursor: sw-resize;
	}

	.se {
		bottom: 0;
		right: 0;
		cursor: se-resize;
	}
</style>

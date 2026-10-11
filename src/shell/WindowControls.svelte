<script lang="ts">
	import { Copy, Minus, Square, X } from '@lucide/svelte'
	import { getCurrentWindow } from '@tauri-apps/api/window'

	import { windowState } from '@/lib/state/window.svelte'
	import Icon from '@/ui/Icon.svelte'

	const win = getCurrentWindow()
	const maximized = $derived(windowState.maximized)
</script>

<div class="controls">
	<button aria-label="Minimize" onclick={() => win.minimize()}
		><Icon icon={Minus} size={14} /></button
	>
	<button aria-label={maximized ? 'Restore' : 'Maximize'} onclick={() => win.toggleMaximize()}>
		<Icon icon={maximized ? Copy : Square} size={12} />
	</button>
	<button class="close" aria-label="Close" onclick={() => win.close()}
		><Icon icon={X} size={16} /></button
	>
</div>

<style>
	.controls {
		display: flex;
		align-self: stretch;
		margin-left: 1px;
	}

	button {
		width: 46px;
		display: grid;
		place-items: center;
	}

	button:hover {
		background: var(--fill);
	}

	.close:hover {
		background: var(--red-fill);
		color: var(--on-red);
	}
</style>

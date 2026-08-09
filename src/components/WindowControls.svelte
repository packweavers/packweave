<script lang="ts">
	import { onMount } from 'svelte'
	import { Copy, Minus, Square, X } from '@lucide/svelte'
	import { getCurrentWindow } from '@tauri-apps/api/window'
	import { tooltip } from '../lib/tooltip'

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
</script>

<div class="flex self-stretch ml-sm">
	<button
		class="w-11 grid place-items-center border-0 bg-transparent text-secondary cursor-pointer hover:bg-button-bg hover:text-contrast"
		aria-label="Minimize"
		use:tooltip={'Minimize'}
		onclick={() => win.minimize()}
	>
		<Minus size={15} />
	</button>
	<button
		class="w-11 grid place-items-center border-0 bg-transparent text-secondary cursor-pointer hover:bg-button-bg hover:text-contrast"
		aria-label={maximized ? 'Restore' : 'Maximize'}
		use:tooltip={maximized ? 'Restore' : 'Maximize'}
		onclick={() => win.toggleMaximize()}
	>
		{#if maximized}
			<Copy size={12} class="-scale-x-100" />
		{:else}
			<Square size={12} />
		{/if}
	</button>
	<button
		class="w-11 grid place-items-center border-0 bg-transparent text-secondary cursor-pointer hover:bg-[#e81123] hover:text-white"
		aria-label="Close"
		use:tooltip={'Close'}
		onclick={() => win.close()}
	>
		<X size={16} />
	</button>
</div>

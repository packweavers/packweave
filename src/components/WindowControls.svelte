<script lang="ts">
	import { Minus, Square, X } from '@lucide/svelte'
	import { getCurrentWindow } from '@tauri-apps/api/window'
	import { tooltip } from '../lib/tooltip'

	const win = getCurrentWindow()
</script>

<div class="flex self-stretch ml-[0.3rem]">
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
		aria-label="Maximize"
		use:tooltip={'Maximize'}
		onclick={() => win.toggleMaximize()}
	>
		<Square size={12} />
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

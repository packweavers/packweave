<script lang="ts">
	import { X } from '@lucide/svelte'
	import type { Snippet } from 'svelte'

	let {
		title,
		onclose,
		children,
	}: { title: string; onclose: () => void; children?: Snippet } = $props()

	function onKey(e: KeyboardEvent) {
		if (e.key === 'Escape') onclose()
	}
</script>

<svelte:window onkeydown={onKey} />

<div
	class="fixed inset-0 z-[60] grid place-items-center bg-black/60 backdrop-blur-[2px]"
	role="presentation"
	onclick={(e) => {
		if (e.target === e.currentTarget) onclose()
	}}
>
	<div
		class="w-[460px] max-w-[calc(100vw-2rem)] max-h-[88vh] flex flex-col bg-bg-super-raised border border-divider rounded-lg shadow-floating"
		role="dialog"
		aria-modal="true"
	>
		<header class="flex items-center justify-between px-5 pt-[1.1rem] pb-2">
			<h2 class="text-[1.1rem]">{title}</h2>
			<button
				class="grid place-items-center p-1 rounded-sm text-secondary hover:text-contrast hover:bg-button-bg"
				aria-label="Close"
				onclick={onclose}
			>
				<X size={18} />
			</button>
		</header>
		<div class="px-5 pt-2 pb-5 overflow-y-auto min-h-0">
			{@render children?.()}
		</div>
	</div>
</div>

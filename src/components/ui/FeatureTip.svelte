<script lang="ts">
	import { Lightbulb, X } from '@lucide/svelte'
	import type { Snippet } from 'svelte'
	import { store } from '../../lib/store.svelte'

	let {
		id,
		children,
		class: cls = '',
	}: { id: string; children?: Snippet; class?: string } = $props()

	const key = $derived(`tip:${id}`)
	const show = $derived(!store.getPref(key, false))
</script>

{#if show}
	<div
		class="flex items-start gap-2.5 bg-brand-highlight border border-divider rounded-md px-3 py-2.5 text-small text-body leading-[1.45] {cls}"
	>
		<Lightbulb size={15} class="text-brand shrink-0 mt-2xs" />
		<div class="min-w-0 flex-1">{@render children?.()}</div>
		<button
			class="shrink-0 grid place-items-center w-6 h-6 -mt-2xs -mr-1 rounded-sm text-secondary bg-transparent border-none cursor-pointer hover:bg-button-bg hover:text-contrast"
			aria-label="Dismiss tip"
			title="Dismiss"
			onclick={() => store.setPref(key, true)}
		>
			<X size={14} />
		</button>
	</div>
{/if}

<script lang="ts">
	import { Info, TriangleAlert, CircleX, CircleCheck } from '@lucide/svelte'
	import type { Snippet } from 'svelte'

	type Kind = 'info' | 'warning' | 'error' | 'success'
	let { type = 'info', title = '', children }: { type?: Kind; title?: string; children?: Snippet } =
		$props()

	const icons = { info: Info, warning: TriangleAlert, error: CircleX, success: CircleCheck }
	const border: Record<Kind, string> = {
		info: 'border-l-blue',
		warning: 'border-l-orange',
		error: 'border-l-red',
		success: 'border-l-green',
	}
	const iconColor: Record<Kind, string> = {
		info: 'text-blue',
		warning: 'text-orange',
		error: 'text-red',
		success: 'text-green',
	}
	const Icon = $derived(icons[type])
</script>

<div
	class="flex gap-2.5 px-[0.9rem] py-[0.8rem] rounded-md border border-divider border-l-[3px] bg-bg-raised {border[
		type
	]}"
>
	<Icon class="shrink-0 mt-0.5 {iconColor[type]}" size={18} />
	<div>
		{#if title}<div class="font-semibold text-contrast text-[0.875rem]">{title}</div>{/if}
		<div class="text-secondary text-[0.8rem] mt-0.5">{@render children?.()}</div>
	</div>
</div>

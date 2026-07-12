<script lang="ts">
	import type { Snippet } from 'svelte'

	type Color = 'gray' | 'brand' | 'red' | 'orange' | 'green' | 'blue' | 'purple'
	let { color = 'gray', children }: { color?: Color; children?: Snippet } = $props()

	const cls: Record<Color, string> = {
		gray: 'bg-surface-4 text-secondary',
		brand: 'bg-brand-highlight text-brand',
		red: 'bg-red/15 text-red',
		orange: 'bg-orange/15 text-orange',
		green: 'bg-brand-highlight text-green',
		blue: 'bg-blue/15 text-blue',
		purple: 'bg-purple/15 text-purple',
	}
</script>

<span
	class="inline-flex items-center gap-1.5 text-[0.72rem] font-semibold leading-none px-2 py-1 rounded-sm whitespace-nowrap {cls[
		color
	]}"
>
	{@render children?.()}
</span>

<script lang="ts">
	import type { Component } from 'svelte'

	interface Props {
		icon: Component<{ size?: number; strokeWidth?: number }>
		size?: 12 | 14 | 16 | 20 | 28
	}

	let { icon: Glyph, size = 16 }: Props = $props()

	const stroke = $derived(size <= 12 ? 2 : size >= 20 ? 1.5 : 1.6)
</script>

<Glyph {size} strokeWidth={stroke} />

<script lang="ts">
	import type { Snippet } from 'svelte'

	interface Props {
		title: string
		detail?: string
		children?: Snippet
	}

	let { title, detail, children }: Props = $props()
</script>

<div class="empty">
	<span class="title-heading">{title}</span>
	{#if detail}
		<span>{detail}</span>
	{/if}
	{#if children}
		<div class="actions">{@render children()}</div>
	{/if}
</div>

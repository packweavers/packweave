<script lang="ts">
	let {
		src = null,
		alt = '',
		size = 40,
	}: { src?: string | null; alt?: string; size?: number } = $props()

	let failed = $state(false)
	const letter = $derived((alt || '?').slice(0, 1).toUpperCase())
</script>

<div
	class="shrink-0 rounded-md overflow-hidden bg-surface-4 grid place-items-center"
	style="width:{size}px;height:{size}px"
>
	{#if src && !failed}
		<img {src} {alt} class="w-full h-full object-cover block" onerror={() => (failed = true)} />
	{:else}
		<span class="font-bold text-secondary" style="font-size:{size * 0.42}px">{letter}</span>
	{/if}
</div>

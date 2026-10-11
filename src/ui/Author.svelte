<script lang="ts">
	interface Props {
		name: string
		icon?: string | null
	}

	let { name, icon = null }: Props = $props()

	let failed = $state(false)

	$effect(() => {
		void icon
		failed = false
	})
</script>

<span class="author">
	{#if icon && !failed}
		<img src={icon} alt="" loading="lazy" onerror={() => (failed = true)} />
	{/if}
	<span class="ellipsis">{name}</span>
</span>

<style>
	.author {
		min-width: 0;
		display: inline-flex;
		align-items: center;
		gap: 5px;
		vertical-align: bottom;
	}

	img {
		width: 14px;
		height: 14px;
		flex: none;
		object-fit: cover;
	}
</style>

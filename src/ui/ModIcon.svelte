<script lang="ts">
	interface Props {
		url?: string | null
		size?: 16 | 20 | 32 | 48
	}

	let { url = null, size = 20 }: Props = $props()

	let failed = $state(false)

	$effect(() => {
		void url
		failed = false
	})
</script>

{#if url && !failed}
	<img
		class="mod-icon"
		src={url}
		alt=""
		width={size}
		height={size}
		loading="lazy"
		onerror={() => (failed = true)}
	/>
{:else}
	<span class="mod-icon block" style:width="{size}px" style:height="{size}px"></span>
{/if}

<style>
	.mod-icon {
		flex: none;
		object-fit: cover;
		image-rendering: auto;
	}

	.block {
		--light: rgba(255, 255, 255, 0.17);
		--dark: rgba(0, 0, 0, 0.22);
		display: inline-block;
		background:
			linear-gradient(var(--light), var(--light)) 0 0 / 25% 25% no-repeat,
			linear-gradient(var(--light), var(--light)) 66.667% 33.333% / 25% 25% no-repeat,
			linear-gradient(var(--light), var(--light)) 33.333% 100% / 25% 25% no-repeat,
			linear-gradient(var(--dark), var(--dark)) 100% 0 / 25% 25% no-repeat,
			linear-gradient(var(--dark), var(--dark)) 33.333% 33.333% / 25% 25% no-repeat,
			linear-gradient(var(--dark), var(--dark)) 0 66.667% / 25% 25% no-repeat,
			linear-gradient(var(--dark), var(--dark)) 100% 100% / 25% 25% no-repeat,
			#6b6a66;
	}
</style>

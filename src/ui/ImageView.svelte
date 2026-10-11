<script lang="ts">
	interface Props {
		src: string
		zoom: number | 'fit'
		onmeasure?: (width: number, height: number) => void
	}

	let { src, zoom, onmeasure }: Props = $props()

	let natural = $state({ width: 0, height: 0 })

	function onload(event: Event): void {
		const image = event.currentTarget as HTMLImageElement
		natural = { width: image.naturalWidth, height: image.naturalHeight }
		onmeasure?.(natural.width, natural.height)
	}

	const style = $derived(
		zoom === 'fit'
			? 'max-width: 100%; max-height: 100%;'
			: `width: ${natural.width * zoom}px; height: ${natural.height * zoom}px;`,
	)
</script>

<div class="stage" class:fit={zoom === 'fit'}>
	<img {src} alt="" {style} class:pixel={natural.width > 0 && natural.width <= 256} {onload} />
</div>

<style>
	.stage {
		flex: 1;
		min-height: 0;
		overflow: auto;
		display: grid;
		place-items: center;
		padding: 24px;
		background-color: var(--doc);
		background-image:
			linear-gradient(45deg, var(--band) 25%, transparent 25%),
			linear-gradient(-45deg, var(--band) 25%, transparent 25%),
			linear-gradient(45deg, transparent 75%, var(--band) 75%),
			linear-gradient(-45deg, transparent 75%, var(--band) 75%);
		background-size: 16px 16px;
		background-position:
			0 0,
			0 8px,
			8px -8px,
			-8px 0;
	}

	.stage.fit img {
		object-fit: contain;
		min-width: 64px;
	}

	.pixel {
		image-rendering: pixelated;
	}
</style>

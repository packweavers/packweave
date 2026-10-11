<script lang="ts">
	import { DOCS } from '@/documents/registry'
	import { layout } from '@/lib/state/layout.svelte'
	import { tabs } from '@/lib/state/tabs.svelte'
	import Empty from '@/ui/Empty.svelte'
	import Splitter from '@/ui/Splitter.svelte'

	const doc = $derived(tabs.active)
	const Pane = $derived(doc ? DOCS[doc.kind].inspector : undefined)
</script>

<aside
	class="inspector"
	style:width="{layout.inspectorWidth}px"
	aria-label="Inspector"
	data-busy-scope={doc ? `inspector:${doc.id}` : undefined}
>
	<div class="edge">
		<Splitter
			axis="x"
			invert
			bind:value={layout.inspectorWidth}
			min={240}
			max={520}
			ondone={() => layout.persist()}
		/>
	</div>
	{#if doc && Pane}
		{#key doc.id}
			<Pane {doc} />
		{/key}
	{:else}
		<Empty title="Nothing selected" />
	{/if}
</aside>

<style>
	.inspector {
		position: relative;
		flex: none;
		display: flex;
		flex-direction: column;
		min-height: 0;
		background: var(--chrome);
		border-left: 1px solid var(--line);
	}

	.edge {
		position: absolute;
		top: 0;
		bottom: 0;
		left: 0;
	}
</style>

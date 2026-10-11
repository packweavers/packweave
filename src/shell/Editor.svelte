<script lang="ts">
	import { docIcon, DOCS, docTitle } from '@/documents/registry'
	import { tabDrag } from '@/lib/state/tabdrag.svelte'
	import { tabs } from '@/lib/state/tabs.svelte'
	import Empty from '@/ui/Empty.svelte'
	import Icon from '@/ui/Icon.svelte'

	import TabStrip from './TabStrip.svelte'
</script>

<div class="editor">
	{#each tabs.groups as group, index (index)}
		{@const doc = group.docs.find((d) => d.id === group.active) ?? null}
		<section
			class="group"
			data-group={index}
			class:split={tabs.groups.length > 1}
			onfocusin={() => (tabs.focused = index)}
			onpointerdown={() => (tabs.focused = index)}
			aria-label="Editor group {index + 1}"
		>
			<TabStrip {group} {index} focused={tabs.focused === index} />
			<div class="document" data-busy-scope={doc?.id}>
				{#if doc}
					{@const Document = DOCS[doc.kind].component}
					{#key doc.id}
						<Document {doc} />
					{/key}
				{:else}
					<Empty title="Nothing open" detail="Open something from the sidebar or Search" />
				{/if}
			</div>
		</section>
	{/each}
</div>

{#if tabDrag.doc && tabDrag.inside}
	<div
		class="ghost"
		class:away={!tabDrag.spot && !tabDrag.panel}
		style:left="{tabDrag.x + 12}px"
		style:top="{tabDrag.y + 10}px"
	>
		<Icon icon={docIcon(tabDrag.doc)} size={14} />
		<span>{docTitle(tabDrag.doc)}</span>
		{#if tabDrag.panel}
			<span class="where">Panel</span>
		{:else if !tabDrag.spot}
			<span class="where">New window</span>
		{/if}
	</div>
{/if}

<style>
	.ghost {
		position: fixed;
		z-index: 90;
		display: flex;
		align-items: center;
		gap: 6px;
		height: 28px;
		padding: 0 12px;
		background: var(--overlay);
		border: 1px solid var(--edge);
		font-size: 12.5px;
		font-weight: 500;
		white-space: nowrap;
		pointer-events: none;
	}

	.ghost.away {
		border-color: var(--focus);
	}

	.where {
		margin-left: 4px;
		color: var(--blue);
		font-size: 12px;
	}

	.editor {
		flex: 1;
		min-height: 0;
		display: flex;
	}

	.group {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
		background: var(--doc);
	}

	.group.split + .group.split {
		border-left: 1px solid var(--line);
	}

	.document {
		position: relative;
		flex: 1;
		min-height: 0;
		display: flex;
		flex-direction: column;
		overflow: hidden;
	}
</style>

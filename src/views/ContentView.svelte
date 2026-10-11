<script lang="ts">
	import { Ellipsis, Plus } from '@lucide/svelte'

	import { FILTER_LABELS, filterCounts, type ContentFilter } from '@/lib/content/rows'
	import { CONTENT_TYPES } from '@/lib/content/types'
	import { findSources, openAddContent, pasteList } from '@/lib/flows/content'
	import { content } from '@/lib/state/content.svelte'
	import { contentView } from '@/lib/state/contentview.svelte'
	import { menus } from '@/lib/state/menus.svelte'
	import { tabs } from '@/lib/state/tabs.svelte'
	import type { ProjectType } from '@/lib/tauri/types'
	import { providerName, typeName } from '@/lib/util/format'
	import IconButton from '@/ui/IconButton.svelte'

	import SideItem from './SideItem.svelte'
	import SideSection from './SideSection.svelte'

	const FILTERS: ContentFilter[] = [
		'updates',
		'uncommitted',
		'problems',
		'dependencies',
		'overrides',
		'client',
		'server',
		'disabled',
		'pinned',
	]

	let openFilters = $state(true)

	const counts = $derived(filterCounts('mod', content.rowInputs))
	const active = $derived(tabs.active)

	function openType(type: ProjectType): void {
		contentView.show(type, 'all')
		tabs.open('content', type, { preview: true })
	}

	function openFilter(filter: ContentFilter): void {
		contentView.show('mod', filter)
		tabs.open('content', 'mod', { preview: true })
	}

	function more(event: MouseEvent): void {
		menus.below(event.currentTarget as Element, [
			{
				label: 'Check for updates',
				command: 'content.check',
				run: () => void content.checkUpdates(true),
			},
			{ label: 'Resolve again', run: () => void content.resolve(true) },
			'separator',
			{ label: 'Paste a list…', run: () => void pasteList() },
			'separator',
			...(['modrinth', 'curseforge'] as const).map((provider) => ({
				label: `Find all on ${providerName(provider)}…`,
				run: () =>
					void findSources(
						content.mods.filter((m) => m.dependencyType !== 'dependency').map((m) => m.key),
						provider,
					),
			})),
		])
	}
</script>

<div class="bar view-header">
	<span>Content</span>
	<span class="spacer"></span>
	<IconButton
		icon={Plus}
		label="Add content"
		command="content.add"
		onclick={() => openAddContent()}
	/>
	<IconButton icon={Ellipsis} label="More" onclick={more} />
</div>
<div class="scroll">
	<div class="types">
		{#each CONTENT_TYPES as type (type)}
			<SideItem
				label={typeName(type, true)}
				count={content.ofType(type).length}
				active={active?.kind === 'content' &&
					active.arg === type &&
					contentView.filter[type] === 'all'}
				onclick={() => openType(type)}
				ondblclick={() => tabs.open('content', type)}
			/>
		{/each}
	</div>
	<SideSection title="Views" bind:open={openFilters}>
		{#each FILTERS as filter (filter)}
			{#if counts[filter] > 0 || filter === 'updates' || filter === 'problems'}
				<SideItem
					label={FILTER_LABELS[filter]}
					count={counts[filter]}
					active={active?.kind === 'content' &&
						active.arg === 'mod' &&
						contentView.filter.mod === filter}
					onclick={() => openFilter(filter)}
				/>
			{/if}
		{/each}
	</SideSection>
</div>

<style>
	.types {
		padding: 4px 0;
	}
</style>

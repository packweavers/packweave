<script lang="ts">
	import { removeContent, toggleDisabled, updateContent } from '@/lib/flows/content'
	import { content } from '@/lib/state/content.svelte'
	import { selection } from '@/lib/state/selection.svelte'
	import type { Doc } from '@/lib/state/tabs.svelte'
	import type { ProjectType } from '@/lib/tauri/types'
	import { listNames, plural, sideName, typeName } from '@/lib/util/format'
	import { pending } from '@/ui/pending'

	import LocalDetails from './LocalDetails.svelte'
	import ModDetails from './ModDetails.svelte'

	interface Props {
		doc: Doc
	}

	let { doc }: Props = $props()

	const type = $derived(doc.arg as ProjectType)
	const mods = $derived(
		selection
			.of(doc.id)
			.map((id) => content.find(id))
			.filter((m) => !!m),
	)
	const locals = $derived(
		selection
			.of(doc.id)
			.map((id) => content.unpublished.find((f) => `local:${f.relPath}` === id))
			.filter((f) => !!f),
	)
	const all = $derived(content.ofType(type))
	const updates = $derived(mods.filter((m) => content.updates[m.key]).length)
	const sides = $derived([...new Set(mods.map((m) => sideName(m.clientSide, m.serverSide)))])
	const enabled = $derived(mods.filter((m) => !m.disabled).length)
</script>

{#if mods.length === 1 && locals.length === 0}
	{#key mods[0].key}
		<ModDetails mod={mods[0]} />
	{/key}
{:else if locals.length === 1 && mods.length === 0}
	{#key locals[0].relPath}
		<LocalDetails file={locals[0]} />
	{/key}
{:else if mods.length > 1}
	<div class="inspector-head">
		<div class="heading">
			<span class="strong"
				>{plural(
					mods.length,
					typeName(type).toLowerCase(),
					typeName(type, true).toLowerCase(),
				)}</span
			>
			<span class="small ellipsis"
				>{listNames(
					mods.map((m) => m.name),
					3,
				)}</span
			>
		</div>
	</div>
	<div class="inspector-body">
		<section class="inspector-section">
			<div class="kv">
				<span class="k">Side</span><span class="v">{sides.length === 1 ? sides[0] : 'Mixed'}</span>
			</div>
			<div class="kv">
				<span class="k">Enabled</span><span class="v"
					>{enabled === mods.length
						? 'All'
						: enabled === 0
							? 'None'
							: `${enabled} of ${mods.length}`}</span
				>
			</div>
		</section>
	</div>
	<div class="bar footer-bar">
		{#if updates > 0}
			<button class="btn" onclick={pending(() => updateContent(mods.map((m) => m.key)))}
				>Update {updates}</button
			>
		{/if}
		<button class="btn" onclick={pending(() => toggleDisabled(mods.map((m) => m.key)))}
			>{enabled > 0 ? `Disable ${enabled}` : `Enable ${mods.length}`}</button
		>
		<span class="spacer"></span>
		<button class="btn danger" onclick={pending(() => removeContent(mods.map((m) => m.key)))}
			>Remove</button
		>
	</div>
{:else}
	<div class="inspector-head">
		<div class="heading">
			<span class="strong">{typeName(type, true)}</span>
			<span class="small">{plural(all.length, 'item', 'items')}</span>
		</div>
	</div>
	<div class="inspector-body">
		<section class="inspector-section">
			<div class="kv">
				<span class="k">In the pack</span><span class="v"
					>{all.filter((m) => m.dependencyType === 'explicit').length}</span
				>
			</div>
			<div class="kv">
				<span class="k">Dependencies</span><span class="v"
					>{all.filter((m) => m.dependencyType === 'dependency').length}</span
				>
			</div>
			<div class="kv">
				<span class="k">Disabled</span><span class="v">{all.filter((m) => m.disabled).length}</span>
			</div>
			<div class="kv">
				<span class="k">Updates</span><span class="v"
					>{all.filter((m) => content.updates[m.key]).length}</span
				>
			</div>
		</section>
	</div>
{/if}

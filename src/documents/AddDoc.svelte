<script lang="ts">
	import { Search } from '@lucide/svelte'
	import { untrack } from 'svelte'

	import { pasteList } from '@/lib/flows/content'
	import { app } from '@/lib/state/app.svelte'
	import { catalog } from '@/lib/state/catalog.svelte'
	import { content } from '@/lib/state/content.svelte'
	import { tabs, type Doc } from '@/lib/state/tabs.svelte'
	import { ws } from '@/lib/state/workspace.svelte'
	import type { CatalogHit, ProjectType, ProviderId } from '@/lib/tauri/types'
	import { compactCount, gameLabel } from '@/lib/util/format'
	import Empty from '@/ui/Empty.svelte'
	import Icon from '@/ui/Icon.svelte'
	import ModIcon from '@/ui/ModIcon.svelte'
	import { pending } from '@/ui/pending'
	import Segmented from '@/ui/Segmented.svelte'

	import DatapackSetup from './content/DatapackSetup.svelte'

	interface Props {
		doc: Doc
	}

	let { doc }: Props = $props()

	let url = $state('')
	let timer: ReturnType<typeof setTimeout> | null = null

	const providers = $derived([
		{ value: 'modrinth' as ProviderId, label: 'Modrinth' },
		...(app.providerReady('curseforge')
			? [{ value: 'curseforge' as ProviderId, label: 'CurseForge' }]
			: []),
		{ value: 'url' as ProviderId, label: 'URL' },
	])

	const type = $derived((doc.arg as ProjectType) || 'mod')

	$effect(() => {
		const next = type
		untrack(() => {
			catalog.type = next
			void catalog.run()
		})
	})

	function research(): void {
		if (timer) clearTimeout(timer)
		timer = setTimeout(() => void catalog.run(), 250)
	}

	function add(hit: CatalogHit): void {
		const held = content.holder(catalog.provider, hit.projectId)
		if (held) {
			if (held.dependencyType === 'dependency') void content.setExplicit(held.key, true)
			return
		}
		void content.add([
			{
				provider: catalog.provider,
				projectId: hit.projectId,
				slug: hit.slug,
				name: hit.title,
				projectType: catalog.type,
			},
		])
	}

	function addUrl(): void {
		const link = url.trim()
		if (!/^https?:\/\//.test(link)) return
		const name = decodeURIComponent(link.split('/').pop() ?? link).replace(/\.(jar|zip)$/i, '')
		void content
			.add([{ provider: 'url', projectId: link, url: link, name, projectType: catalog.type }])
			.then((ok) => {
				if (ok) url = ''
			})
	}

	function onscroll(event: Event): void {
		const el = event.currentTarget as HTMLElement
		const nearEnd = el.scrollTop + el.clientHeight > el.scrollHeight - 200
		if (nearEnd && catalog.more && !catalog.loading) void catalog.run(true)
	}
</script>

<div class="bar doc-toolbar">
	<span class="doc-title">Add content</span>
	<Segmented
		small
		options={providers}
		bind:value={catalog.provider}
		onchange={() => void catalog.run()}
	/>
	<Segmented
		small
		options={[
			{ value: 'mod', label: 'Mods' },
			{ value: 'resourcepack', label: 'Resource packs' },
			{ value: 'shader', label: 'Shaders' },
			{ value: 'datapack', label: 'Data packs' },
		]}
		value={type}
		onchange={(next) => tabs.setArg(doc.id, next)}
	/>
	<span class="spacer"></span>
	<button class="btn" onclick={pending(() => pasteList())}>Paste a list…</button>
</div>

{#if type === 'datapack' && !ws.manifest?.datapacks}
	<DatapackSetup />
{:else if catalog.provider === 'url'}
	<div class="url-form">
		<input
			class="field"
			bind:value={url}
			placeholder="https://example.com/my-mod.jar"
			spellcheck="false"
			onkeydown={(e) => e.key === 'Enter' && addUrl()}
		/>
		<button class="btn primary" disabled={!/^https?:\/\//.test(url.trim())} onclick={addUrl}
			>Add</button
		>
	</div>
{:else}
	<div class="search-row">
		<Icon icon={Search} size={14} />
		<input
			bind:value={catalog.query}
			oninput={research}
			placeholder="Search {catalog.provider === 'modrinth' ? 'Modrinth' : 'CurseForge'}"
			spellcheck="false"
		/>
		{#if ws.manifest}<span class="small"
				>{gameLabel(ws.manifest.minecraft, ws.manifest.loader, null)}</span
			>{/if}
	</div>
	{#if catalog.error}
		<div class="banner error">
			Couldn't reach {catalog.provider === 'modrinth' ? 'Modrinth' : 'CurseForge'}
			<span class="spacer"></span>
			<button class="btn" onclick={pending(() => catalog.run())}>Retry</button>
		</div>
	{/if}
	<div class="scroll" {onscroll}>
		{#each catalog.results as hit (hit.projectId)}
			{@const held = content.holder(catalog.provider, hit.projectId)}
			{@const inPack = !!held && held.dependencyType !== 'dependency'}
			<div
				class="trow tall"
				class:selected={catalog.selected?.projectId === hit.projectId}
				role="option"
				aria-selected={catalog.selected?.projectId === hit.projectId}
				tabindex="-1"
				onclick={() => (catalog.selected = hit)}
				ondblclick={() => (held && inPack ? tabs.open('mod', held.key) : add(hit))}
				onkeydown={(e) => e.key === 'Enter' && (catalog.selected = hit)}
			>
				<ModIcon url={hit.iconUrl} size={32} />
				<div class="stack">
					<span class="medium ellipsis">{hit.title}</span>
					<span class="small ellipsis">by {hit.author} · {hit.description}</span>
				</div>
				<span class="mono small downloads">{compactCount(hit.downloads)}</span>
				{#if inPack}
					<span class="tag in">in pack</span>
				{:else}
					{#if held}<span class="tag">dependency</span>{/if}
					<button
						class="btn"
						onclick={(e) => {
							e.stopPropagation()
							add(hit)
						}}
					>
						Add
					</button>
				{/if}
			</div>
		{:else}
			{#if !catalog.loading}
				<Empty title="No results" />
			{/if}
		{/each}
		{#if catalog.loading}
			<div class="trow"><span class="spinner"></span></div>
		{/if}
	</div>
{/if}

<style>
	.url-form,
	.search-row {
		flex: none;
		display: flex;
		align-items: center;
		gap: 8px;
		height: 40px;
		padding-left: 16px;
		border-bottom: 1px solid var(--line);
	}

	.url-form .field {
		flex: 1;
		height: 100%;
	}

	.url-form .btn {
		align-self: stretch;
		height: auto;
	}

	.search-row {
		padding-right: 16px;
	}

	.search-row input {
		flex: 1;
		min-width: 0;
		height: 100%;
		font-size: 13px;
	}

	.tall {
		height: 52px;
	}

	.stack {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
		gap: 2px;
	}

	.downloads {
		flex: 0 0 48px;
		text-align: right;
	}

	.in {
		padding: 0 14px;
	}
</style>

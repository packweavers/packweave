<script lang="ts">
	import { curseforgePage } from '@/lib/content/sources'
	import { versionLoader } from '@/lib/content/types'
	import { catalog } from '@/lib/state/catalog.svelte'
	import { content, suggestedItem } from '@/lib/state/content.svelte'
	import { tabs, type Doc } from '@/lib/state/tabs.svelte'
	import { tasks } from '@/lib/state/tasks.svelte'
	import { ws } from '@/lib/state/workspace.svelte'
	import { versions as loadVersions } from '@/lib/tauri/catalog'
	import { previewChange } from '@/lib/tauri/content'
	import type { AddItem, ImpactReport, VersionInfo } from '@/lib/tauri/types'
	import { openExternal } from '@/lib/tauri/windows'
	import { listNames, typeName } from '@/lib/util/format'
	import Empty from '@/ui/Empty.svelte'
	import ModIcon from '@/ui/ModIcon.svelte'

	interface Props {
		doc: Doc
	}

	let {}: Props = $props()

	const hit = $derived(catalog.selected)
	const existing = $derived(hit ? content.holder(catalog.provider, hit.projectId) : undefined)
	const inPack = $derived(!!existing && existing.dependencyType !== 'dependency')
	const asDependency = $derived(existing?.dependencyType === 'dependency')
	const requiredBy = $derived(existing && asDependency ? content.requiredBy(existing.key) : [])

	let latest = $state<VersionInfo | null>(null)
	let impact = $state<ImpactReport | null>(null)
	let picked = $state<string[]>([])

	const item = $derived<AddItem | null>(
		hit
			? {
					provider: catalog.provider,
					projectId: hit.projectId,
					slug: hit.slug,
					name: hit.title,
					projectType: catalog.type,
				}
			: null,
	)

	const minecraft = $derived(ws.manifest?.minecraft ?? null)
	const loader = $derived(ws.manifest?.loader ?? null)

	$effect(() => {
		const current = hit
		const adding = item
		const dir = ws.dir
		const provider = catalog.provider
		const forLoader = versionLoader(catalog.type, loader)
		const fresh = !existing
		latest = null
		impact = null
		picked = []
		if (!current || !adding || !minecraft || !dir || provider === 'url') return
		void loadVersions(current.projectId, provider, minecraft, forLoader)
			.then((list) => {
				if (catalog.selected !== current) return
				latest = list.find((v) => v.versionType === 'release') ?? list[0] ?? null
			})
			.catch((error) => tasks.fail(`Loading versions of ${current.title}`, error))
		if (!fresh) return
		void previewChange(dir, { add: [adding] })
			.then((report) => {
				if (catalog.selected === current) impact = report
			})
			.catch((error) => tasks.fail('Checking what else changes', error))
	})

	const brought = $derived(
		(impact?.changes ?? []).filter((c) => c.projectId !== hit?.projectId && c.kind === 'added'),
	)
	const optional = $derived(impact?.optional ?? [])

	function pick(projectId: string, on: boolean): void {
		picked = on ? [...picked, projectId] : picked.filter((id) => id !== projectId)
	}

	function add(adding: AddItem): void {
		const extras = optional.filter((s) => picked.includes(s.projectId)).map(suggestedItem)
		void content.add([adding, ...extras])
	}

	function webPage(): string {
		if (!hit) return ''
		return catalog.provider === 'curseforge'
			? curseforgePage(catalog.type, hit.slug)
			: `https://modrinth.com/${hit.projectType}/${hit.slug}`
	}
</script>

{#if hit && item}
	<div class="inspector-head">
		<ModIcon url={hit.iconUrl} size={32} />
		<div class="heading">
			<span class="strong ellipsis">{hit.title}</span>
			<span class="small ellipsis">{typeName(catalog.type)} · by {hit.author}</span>
		</div>
	</div>
	<div class="inspector-body">
		<p>{hit.description}</p>
		<section class="inspector-section">
			<div class="band">Version</div>
			<div class="kv">
				<span class="v mono">{latest?.versionNumber ?? '…'}</span>
				{#if latest}<span class="tag" class:changed={latest.versionType !== 'release'}
						>{latest.versionType}</span
					>{/if}
			</div>
		</section>
		{#if asDependency}
			<section class="inspector-section">
				<div class="band">In the pack</div>
				<span class="small"
					>As a dependency{requiredBy.length
						? ` of ${listNames(
								requiredBy.map((m) => m.name),
								3,
							)}`
						: ''}. Add it to keep it even if nothing requires it.</span
				>
			</section>
		{:else if !inPack}
			<section class="inspector-section">
				<div class="band">Dependencies</div>
				{#if !impact}
					<span class="spinner"></span>
				{:else if brought.length + optional.length === 0}
					<span class="small">None</span>
				{/if}
				{#each brought as dep (dep.projectId)}
					<div class="item-row">
						<span class="ellipsis">{dep.name}</span>
						<span class="tag">required</span>
						<span class="spacer"></span>
						<span class="tag added">added too</span>
					</div>
				{/each}
				{#each optional as dep (dep.projectId)}
					<label class="item-row">
						<input
							type="checkbox"
							checked={picked.includes(dep.projectId)}
							onchange={(e) => pick(dep.projectId, e.currentTarget.checked)}
						/>
						<span class="ellipsis">{dep.name}</span>
						<span class="tag">optional</span>
					</label>
				{/each}
				{#each impact?.problems ?? [] as problem (problem)}
					<span><span class="removed">✗</span> {problem}</span>
				{/each}
			</section>
		{/if}
	</div>
	<div class="bar footer-bar">
		{#if inPack}
			<button class="btn" onclick={() => existing && tabs.open('mod', existing.key)}>Open</button>
		{:else if asDependency}
			<button
				class="btn primary"
				onclick={() => existing && void content.setExplicit(existing.key, true)}>Add</button
			>
			<button class="btn" onclick={() => existing && tabs.open('mod', existing.key)}>Open</button>
		{:else}
			<button class="btn primary" onclick={() => add(item)}
				>{picked.length > 0 ? `Add with ${picked.length} optional` : 'Add'}</button
			>
		{/if}
		<span class="spacer"></span>
		<button class="btn quiet" onclick={() => void openExternal(webPage())}>Page</button>
	</div>
{:else}
	<Empty title="Nothing selected" />
{/if}

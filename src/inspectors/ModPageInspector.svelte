<script lang="ts">
	import { contentChange } from '@/lib/content/changes'
	import { versionLoader } from '@/lib/content/types'
	import { updatePins } from '@/lib/flows/content'
	import { changeMark } from '@/lib/git/changes'
	import { content } from '@/lib/state/content.svelte'
	import { selection } from '@/lib/state/selection.svelte'
	import type { Doc } from '@/lib/state/tabs.svelte'
	import { tasks } from '@/lib/state/tasks.svelte'
	import { ws } from '@/lib/state/workspace.svelte'
	import { versions as loadVersions, versionChangelog } from '@/lib/tauri/catalog'
	import { previewChange } from '@/lib/tauri/content'
	import type { ImpactReport, VersionInfo } from '@/lib/tauri/types'
	import Markdown from '@/ui/Markdown.svelte'
	import { pending } from '@/ui/pending'

	import ModDetails from './ModDetails.svelte'

	interface Props {
		doc: Doc
	}

	let { doc }: Props = $props()

	const mod = $derived(content.find(doc.arg))
	const previewId = $derived(selection.of(doc.id)[0] ?? null)

	let version = $state<VersionInfo | null>(null)
	let impact = $state<ImpactReport | null>(null)
	let changelog = $state('')
	let loading = $state(false)

	const provider = $derived(mod?.preferred ?? null)
	const projectType = $derived(mod?.projectType ?? null)
	const minecraft = $derived(ws.manifest?.minecraft ?? null)
	const loader = $derived(ws.manifest?.loader ?? null)

	$effect(() => {
		const id = previewId
		const projectId = mod?.projectId
		const key = mod?.key
		const source = provider
		const dir = ws.dir
		const forLoader = projectType ? versionLoader(projectType, loader) : null
		version = null
		impact = null
		changelog = ''
		if (!id || !projectId || !key || !source || !dir || !minecraft) return
		loading = true
		void loadVersions(projectId, source, minecraft, forLoader)
			.then((list) => {
				if (previewId === id) version = list.find((v) => v.id === id) ?? null
			})
			.catch((error) => tasks.fail(`Loading versions of ${mod?.name ?? projectId}`, error))
		void previewChange(dir, { versions: [{ key, version: id }] })
			.then((report) => {
				if (previewId === id) impact = report
			})
			.catch((error) => tasks.fail('Checking what else changes', error))
			.finally(() => {
				if (previewId === id) loading = false
			})
		void versionChangelog(source, projectId, id)
			.then((text) => {
				if (previewId === id) changelog = text
			})
			.catch((error) => tasks.fail('Loading the changelog', error))
	})

	const others = $derived((impact?.changes ?? []).filter((c) => c.key !== mod?.key))
</script>

{#if mod && previewId}
	<div class="inspector-head">
		<div class="heading">
			<span class="strong mono">{version?.versionNumber ?? '…'}</span>
			<span class="words">
				{#if version}<span class="tag" class:changed={version.versionType !== 'release'}
						>{version.versionType}</span
					>{/if}
				<span class="tag info">preview</span>
			</span>
		</div>
	</div>
	<div class="inspector-body">
		<section class="inspector-section">
			<div class="band">Also changes</div>
			{#if loading}
				<span class="spinner"></span>
			{:else}
				{#each others as change (change.key)}
					<div class="item-row">
						<span class="mono">{changeMark(contentChange(change.kind))}</span>
						<span class="ellipsis">{change.name}</span>
						<span class="spacer"></span>
						<span class="mono small"
							>{change.fromVersion ?? ''}{change.fromVersion && change.toVersion
								? ' → '
								: ''}{change.toVersion ?? ''}</span
						>
					</div>
				{:else}
					<span class="small">Nothing else</span>
				{/each}
				{#if impact && impact.problems.length === 0}
					<span><span class="added">✓</span> no conflicts</span>
				{/if}
				{#each impact?.problems ?? [] as problem (problem)}
					<span><span class="removed">✗</span> {problem}</span>
				{/each}
			{/if}
		</section>
		<section class="inspector-section">
			<div class="band">Changelog</div>
			{#if changelog}
				<Markdown source={changelog} html={provider === 'curseforge'} />
			{:else}
				<span class="small">No changelog</span>
			{/if}
		</section>
	</div>
	<div class="bar footer-bar">
		<button
			class="btn primary"
			onclick={pending(() => updatePins([{ key: mod.key, version: previewId }]))}>Switch</button
		>
	</div>
{:else if mod}
	{#key mod.key}
		<ModDetails {mod} showOpen={false} />
	{/key}
{/if}

<style>
	.words {
		display: flex;
		gap: 8px;
	}
</style>

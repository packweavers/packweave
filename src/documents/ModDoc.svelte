<script lang="ts">
	import { onMount } from 'svelte'

	import { isPlatform, platformPage, PLATFORMS, type Platform } from '@/lib/content/sources'
	import { versionLoader } from '@/lib/content/types'
	import { findSource, removeContent, toggleDisabled } from '@/lib/flows/content'
	import { content } from '@/lib/state/content.svelte'
	import { selection } from '@/lib/state/selection.svelte'
	import { tabs, type Doc } from '@/lib/state/tabs.svelte'
	import { tasks } from '@/lib/state/tasks.svelte'
	import { ws } from '@/lib/state/workspace.svelte'
	import { errorText } from '@/lib/tauri/call'
	import { versions as loadVersions, projectDetails } from '@/lib/tauri/catalog'
	import { modConfigs } from '@/lib/tauri/files'
	import type { ProjectDetails, ProviderId, VersionInfo } from '@/lib/tauri/types'
	import { openExternal } from '@/lib/tauri/windows'
	import { ago, providerName, sideName } from '@/lib/util/format'
	import Author from '@/ui/Author.svelte'
	import Empty from '@/ui/Empty.svelte'
	import ModIcon from '@/ui/ModIcon.svelte'
	import { pending } from '@/ui/pending'
	import Select from '@/ui/Select.svelte'

	interface Props {
		doc: Doc
	}

	let { doc }: Props = $props()

	const mod = $derived(content.find(doc.arg))
	const key = $derived(mod?.key ?? doc.arg)
	const source = $derived(mod?.sources[mod.preferred])
	const change = $derived(content.changeWordOf(key))
	const preview = $derived(selection.of(doc.id)[0] ?? null)
	const required = $derived(content.bringsIn(key))
	const optional = $derived(content.optionalIn(key).filter((m) => !required.includes(m)))
	const missing = $derived(content.optionalMissing(key))
	const alternates = $derived(
		mod
			? PLATFORMS.filter((platform) => platform !== mod.preferred && !!mod.sources[platform]?.id)
					.map((platform) => `${platform}:${mod.sources[platform]?.id}`)
					.join(' ')
			: '',
	)

	let details = $state<ProjectDetails | null>(null)
	let versionList = $state<VersionInfo[]>([])
	let showBetas = $state(ws.manifest?.channel !== 'release')
	let configs = $state<string[]>([])
	let addingSource = $state(false)
	let altVersions = $state<Record<string, VersionInfo[]>>({})
	let versionError = $state<string | null>(null)

	const shownVersions = $derived(
		versionList
			.filter((v) => showBetas || v.versionType === 'release' || v.id === source?.versionId)
			.slice(0, 60),
	)

	const provider = $derived(mod?.preferred ?? null)
	const projectType = $derived(mod?.projectType ?? null)
	const minecraft = $derived(ws.manifest?.minecraft ?? null)
	const loader = $derived(ws.manifest?.loader ?? null)

	$effect(() => {
		if (mod && mod.key !== doc.arg) tabs.rekey(doc.id, mod.key)
	})

	$effect(() => {
		const id = mod?.projectId
		if (!id || !minecraft || (provider !== 'modrinth' && provider !== 'curseforge')) return
		void projectDetails(provider, id)
			.then((d) => (details = d))
			.catch((error) => {
				details = null
				tasks.fail(`Loading the ${providerName(provider)} page`, error)
			})
		void loadVersions(
			id,
			provider,
			minecraft,
			projectType ? versionLoader(projectType, loader) : null,
		)
			.then((list) => {
				versionList = list
				versionError = null
			})
			.catch((error) => {
				versionList = []
				versionError = errorText(error)
			})
	})

	$effect(() => {
		if (!minecraft || !alternates) return
		const kind = projectType ? versionLoader(projectType, loader) : null
		for (const entry of alternates.split(' ')) {
			const [platform, id] = entry.split(':') as [ProviderId, string]
			void loadVersions(id, platform, minecraft, kind)
				.then((list) => (altVersions = { ...altVersions, [platform]: list.slice(0, 60) }))
				.catch((error) => {
					altVersions = { ...altVersions, [platform]: [] }
					tasks.fail(`Loading ${providerName(platform)} versions`, error)
				})
		}
	})

	onMount(() => {
		const dir = ws.dir
		if (!dir) return
		void modConfigs(dir, key)
			.then((found) => (configs = found))
			.catch((error) => {
				configs = []
				tasks.fail('Finding config files', error)
			})
	})

	function choose(version: VersionInfo): void {
		selection.set(doc.id, version.id === source?.versionId ? [] : [version.id])
	}

	async function addSource(target: ProviderId): Promise<void> {
		if (!mod) return
		addingSource = true
		try {
			await findSource(mod.key, target)
		} finally {
			addingSource = false
		}
	}
</script>

{#if !mod}
	<Empty title="Not in this pack">
		<button class="btn" onclick={() => tabs.close(doc.id)}>Close</button>
	</Empty>
{:else}
	<div class="bar doc-toolbar">
		<span class="doc-title">{mod.name}</span>
		<span class="spacer"></span>
		<button class="btn" onclick={pending(() => toggleDisabled([mod.key]))}
			>{mod.disabled ? 'Enable' : 'Disable'}</button
		>
		<button class="btn danger" onclick={pending(() => removeContent([mod.key]))}>Remove</button>
	</div>
	<div class="scroll page">
		<header class="head">
			<ModIcon url={content.meta[mod.projectId]?.iconUrl ?? details?.iconUrl} size={48} />
			<div class="titles">
				<h1 class="title-doc">{mod.name}</h1>
				<span>
					{#if content.meta[mod.projectId]?.author}<Author
							name={content.meta[mod.projectId].author}
							icon={content.meta[mod.projectId].authorIconUrl}
						/> ·
					{/if}
					{sideName(mod.clientSide, mod.serverSide)} · {providerName(mod.preferred)}
					{#if change}<span class="tag {change}">{change}</span>{/if}
					{#if mod.dependencyType === 'dependency'}<span class="tag">dependency</span>{/if}
				</span>
			</div>
			{#if details}
				<button class="btn quiet" onclick={() => void openExternal(details!.url)}
					>{providerName(mod.preferred)} page</button
				>
			{/if}
		</header>
		{#if details?.description}<p class="description">{details.description}</p>{/if}

		<section>
			<div class="band bar">
				Versions
				<span class="spacer"></span>
				<label class="check small"
					><input type="checkbox" class="switch" bind:checked={showBetas} />Betas</label
				>
			</div>
			<div class="thead">
				<span class="c-version">Version</span>
				<span class="c-channel">Channel</span>
				<span class="c-date">Published</span>
				<span class="c-state"></span>
			</div>
			{#each shownVersions as version (version.id)}
				{@const current = version.id === source?.versionId}
				<button
					class="trow"
					class:selected={preview === version.id}
					onclick={() => choose(version)}
				>
					<span class="c-version mono">{version.versionNumber}</span>
					<span class="c-channel" class:changed={version.versionType !== 'release'}
						>{version.versionType}</span
					>
					<span class="c-date">{ago(version.datePublished)}</span>
					<span class="c-state">
						{#if current}<span class="tag">in pack</span>{:else if preview === version.id}<span
								class="tag info">preview</span
							>{/if}
					</span>
				</button>
			{:else}
				<div class="trow">
					{versionError
						? `Couldn't load versions: ${versionError}`
						: 'No versions for this game version'}
				</div>
			{/each}
		</section>

		<section>
			<div class="band bar">Sources</div>
			{#each Object.entries(mod.sources) as [provider, file] (provider)}
				{@const page = isPlatform(provider as ProviderId)
					? platformPage(mod, provider as Platform)
					: null}
				<div class="trow">
					<span class="c-platform">{providerName(provider as ProviderId)}</span>
					<span class="c-linked mono">
						{#if page}
							<button
								class="linked"
								title="Open on {providerName(provider as ProviderId)}"
								onclick={() => void openExternal(page)}>{file?.slug || file?.id}</button
							>
						{:else}
							<span class="ellipsis">{file?.filename || file?.url || ''}</span>
						{/if}
					</span>
					{#if provider === mod.preferred}
						<span class="mono">{file?.versionNumber ?? ''}</span>
					{:else}
						<Select
							options={[
								{ value: '', label: `Match ${providerName(mod.preferred)}` },
								...(file?.pinned &&
								file.versionId &&
								!(altVersions[provider] ?? []).some((v) => v.id === file.versionId)
									? [{ value: file.versionId, label: file.versionNumber || file.versionId }]
									: []),
								...(altVersions[provider] ?? []).map((v) => ({
									value: v.id,
									label: `${v.versionNumber}${v.versionType !== 'release' ? ` · ${v.versionType}` : ''}`,
								})),
							]}
							value={file?.pinned ? (file.versionId ?? '') : ''}
							mono
							onchange={(next) =>
								void content.setSourceVersion(mod.key, provider as ProviderId, next || null)}
						/>
						{#if !file?.pinned && file?.versionNumber}<span class="mono small hint"
								>{file.versionNumber}</span
							>{:else if !file?.pinned && (altVersions[provider] ?? []).length > 0}<span
								class="small hint">no match</span
							>{/if}
					{/if}
					<span class="spacer"></span>
					{#if provider === mod.preferred}
						<span class="tag">in use</span>
					{:else}
						<button
							class="btn"
							disabled={!file?.versionNumber}
							title={file?.versionNumber
								? undefined
								: (altVersions[provider] ?? []).length > 0
									? 'No matching version'
									: 'No version for this game version'}
							onclick={pending(() => content.preferSource([mod.key], provider as ProviderId))}
							>Use</button
						>
						<button
							class="btn danger"
							onclick={pending(() => content.removeSource(mod.key, provider as ProviderId))}
							>Remove</button
						>
					{/if}
				</div>
			{/each}
			{#each PLATFORMS.filter((p) => !mod.sources[p]) as missing (missing)}
				<div class="trow">
					<span class="c-platform">{providerName(missing)}</span>
					<span class="c-linked hint">not linked</span>
					<span class="spacer"></span>
					<button class="btn" disabled={addingSource} onclick={pending(() => addSource(missing))}
						>Find on {providerName(missing)}</button
					>
				</div>
			{/each}
		</section>

		{#if required.length + optional.length + missing.length > 0}
			<section>
				<div class="band bar">Dependencies</div>
				{#each [...required, ...optional] as dep (dep.key)}
					<button class="trow" onclick={() => tabs.open('mod', dep.key)}>
						<ModIcon url={content.meta[dep.projectId]?.iconUrl} size={16} />
						<span class="name">{dep.name}</span>
						<span class="tag">{required.includes(dep) ? 'required' : 'optional'}</span>
						<span class="tag added">✓ in pack</span>
						<span class="spacer"></span>
						<span class="mono">{dep.sources[dep.preferred]?.versionNumber ?? ''}</span>
					</button>
				{/each}
				{#each missing as dep (dep.projectId)}
					<div class="trow">
						<ModIcon size={16} />
						<span class="name">{dep.name}</span>
						<span class="tag">optional</span>
						<span class="spacer"></span>
						<button class="btn" onclick={pending(() => content.addSuggested(dep))}>Add</button>
					</div>
				{/each}
			</section>
		{/if}

		<section>
			<div class="band bar">Configs</div>
			{#each configs as rel (rel)}
				<button class="trow" onclick={() => tabs.open('file', rel)}>
					<span class="mono name">{rel.replace(/^overrides\//, '')}</span>
				</button>
			{:else}
				<div class="trow">Not created yet</div>
			{/each}
		</section>
	</div>
{/if}

<style>
	.page {
		padding-bottom: 32px;
	}

	.head {
		display: flex;
		align-items: center;
		gap: 14px;
		padding: 20px 16px 10px;
	}

	.titles {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
		gap: 4px;
	}

	.titles span {
		display: flex;
		align-items: center;
		gap: 8px;
	}

	.description {
		padding: 0 16px 16px;
		max-width: 760px;
		line-height: 1.5;
	}

	section {
		margin-top: 12px;
	}

	.trow {
		width: 100%;
		text-align: left;
	}

	.c-version {
		flex: 0 0 200px;
	}

	.c-channel {
		flex: 0 0 90px;
	}

	.c-date {
		flex: 0 0 140px;
	}

	.c-state {
		flex: 1;
	}

	.c-platform {
		flex: 0 0 110px;
	}

	.c-linked {
		flex: 0 1 220px;
		min-width: 0;
		display: flex;
	}

	.linked {
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		color: var(--grey);
	}

	.linked:hover {
		color: var(--text);
		text-decoration: underline;
	}
</style>

<script lang="ts">
	import { ArrowUp, Ellipsis, Pin } from '@lucide/svelte'

	import { isPlatform, PLATFORMS } from '@/lib/content/sources'
	import { versionLoader } from '@/lib/content/types'
	import { findSources, removeContent, updatePins } from '@/lib/flows/content'
	import { SIDE_WORDS } from '@/lib/instance/items'
	import { contentMenu, sourceMenu } from '@/lib/menus'
	import { app } from '@/lib/state/app.svelte'
	import { content } from '@/lib/state/content.svelte'
	import { instance } from '@/lib/state/instance.svelte'
	import { menus } from '@/lib/state/menus.svelte'
	import { tabs } from '@/lib/state/tabs.svelte'
	import { tasks } from '@/lib/state/tasks.svelte'
	import { ws } from '@/lib/state/workspace.svelte'
	import { versions as loadVersions } from '@/lib/tauri/catalog'
	import type { LockedMod, ProviderId, VersionInfo } from '@/lib/tauri/types'
	import { listNames, providerName, sideName } from '@/lib/util/format'
	import Icon from '@/ui/Icon.svelte'
	import IconButton from '@/ui/IconButton.svelte'
	import ModIcon from '@/ui/ModIcon.svelte'
	import { pending } from '@/ui/pending'
	import Select from '@/ui/Select.svelte'

	import ConfigList from './ConfigList.svelte'
	import UpdateNotes from './UpdateNotes.svelte'

	interface Props {
		mod: LockedMod
		showOpen?: boolean
	}

	let { mod, showOpen = true }: Props = $props()

	let versionList = $state<VersionInfo[]>([])

	const source = $derived(mod.sources[mod.preferred])
	const update = $derived(content.updates[mod.key] ?? null)
	const change = $derived(content.changeWordOf(mod.key))
	const requiredBy = $derived(content.requiredBy(mod.key))
	const required = $derived(content.bringsIn(mod.key))
	const optional = $derived(content.optionalIn(mod.key).filter((m) => !required.includes(m)))
	const missing = $derived(content.optionalMissing(mod.key))
	const syncKey = $derived(`mod:${mod.key}`)
	const syncItem = $derived(instance.items.find((i) => i.key === syncKey))
	const syncRule = $derived(instance.ignoredEntryFor(syncKey))
	const instanceWord = $derived.by(() => {
		if (instance.running) return 'running'
		if (syncRule) return 'ignored'
		return syncItem ? SIDE_WORDS[syncItem.side] : 'in sync'
	})
	const others = $derived((Object.keys(mod.sources) as ProviderId[]).filter((p) => !isPlatform(p)))
	const versionOptions = $derived.by(() => {
		const list = versionList.map((v) => ({
			value: v.id,
			label: `${v.versionNumber}${v.versionType !== 'release' ? ` · ${v.versionType}` : ''}`,
		}))
		const current = source?.versionId
		if (current && !list.some((o) => o.value === current)) {
			list.unshift({ value: current, label: source?.versionNumber ?? current })
		}
		return list
	})
	const provider = $derived(mod.preferred)
	const projectId = $derived(mod.projectId)
	const projectType = $derived(mod.projectType)
	const minecraft = $derived(ws.manifest?.minecraft ?? null)
	const loader = $derived(ws.manifest?.loader ?? null)

	$effect(() => {
		if (!minecraft || (provider !== 'modrinth' && provider !== 'curseforge')) return
		void loadVersions(projectId, provider, minecraft, versionLoader(projectType, loader))
			.then((list) => (versionList = list))
			.catch((error) => {
				versionList = []
				tasks.fail(`Loading versions of ${mod.name}`, error)
			})
	})

	function pickVersion(version: string): void {
		if (version !== source?.versionId) void updatePins([{ key: mod.key, version }])
	}
</script>

<div class="inspector-head">
	<ModIcon url={content.meta[mod.projectId]?.iconUrl} size={32} />
	<div class="heading">
		<span class="strong ellipsis">{mod.name}</span>
		<span class="small ellipsis mono version"
			>{#if mod.pinned}<Icon icon={Pin} size={12} />{/if}{source?.versionNumber ?? ''} · {providerName(
				mod.preferred,
			)}</span
		>
		{#if update}
			<span class="small ellipsis mono update" title={update.versionNumber}
				><Icon icon={ArrowUp} size={12} />{update.versionNumber}</span
			>
		{/if}
		{#if change}
			<span class="words">
				<span class="tag {change}">{change}</span>
				<span class="tag">uncommitted</span>
			</span>
		{/if}
	</div>
</div>
<div class="inspector-body">
	{#if update}
		<section class="inspector-section">
			<div class="band">
				Changelog<span class="spacer"></span><span class="small mono band-detail"
					>{source?.versionNumber ?? '?'} → {update.versionNumber}</span
				>
			</div>
			<UpdateNotes projectId={mod.projectId} version={update.versionId} />
		</section>
	{/if}
	<section class="inspector-section">
		<div class="band">Details</div>
		<div class="kv">
			<span class="k">Side</span><span class="v">{sideName(mod.clientSide, mod.serverSide)}</span>
		</div>
		<div class="kv">
			<span class="k">File</span><span class="v mono">{source?.filename ?? '—'}</span>
		</div>
		<div class="kv">
			<span class="k">Required by</span><span class="v"
				>{requiredBy.length
					? listNames(
							requiredBy.map((m) => m.name),
							3,
						)
					: '—'}</span
			>
		</div>
	</section>
	<section class="inspector-section">
		<div class="band">Sources</div>
		{#each PLATFORMS as platform (platform)}
			{@const file = mod.sources[platform]}
			{#if file}
				<div class="kv">
					<span class="k">{providerName(platform)}</span>
					<span class="v mono" class:hint={!file.versionNumber}
						>{file.versionNumber || 'no matching version'}</span
					>
					{#if file.pinned}<span class="tag">pinned</span>{/if}
					<span class="spacer"></span>
					{#if platform === mod.preferred}
						<span class="tag">in use</span>
					{:else}
						<button
							class="btn small"
							disabled={!file.versionNumber}
							onclick={pending(() => content.preferSource([mod.key], platform))}>Use</button
						>
					{/if}
					<IconButton
						icon={Ellipsis}
						label="{providerName(platform)} options"
						small
						onclick={(e) => menus.below(e.currentTarget as Element, sourceMenu(mod, platform))}
					/>
				</div>
				<span class="small mono linked" title={file.slug || file.id}>{file.slug || file.id}</span>
			{:else if app.providerReady(platform)}
				<div class="kv">
					<span class="k">{providerName(platform)}</span>
					<span class="v hint">not linked</span>
					<span class="spacer"></span>
					<button class="btn small" onclick={pending(() => findSources([mod.key], platform))}
						>Find…</button
					>
				</div>
			{/if}
		{/each}
		{#each others as provider (provider)}
			<div class="kv">
				<span class="k">{providerName(provider)}</span>
				<span class="v mono">{mod.sources[provider]?.filename ?? '—'}</span>
			</div>
		{/each}
	</section>
	<section class="inspector-section">
		<div class="band">Settings</div>
		<div class="kv">
			<span class="k">Version</span>
			{#if versionOptions.length > 0}
				<Select
					options={versionOptions}
					value={source?.versionId ?? ''}
					mono
					class="grow"
					onchange={pickVersion}
				/>
			{:else}
				<span class="v mono">{source?.versionNumber ?? '—'}</span>
			{/if}
		</div>
		<label class="kv">
			<span class="k">Enabled</span>
			<input
				type="checkbox"
				class="switch"
				checked={!mod.disabled}
				onchange={() => void content.setDisabled([mod.key], !mod.disabled)}
			/>
		</label>
		<label class="kv">
			<span class="k">Pin version</span>
			<input
				type="checkbox"
				class="switch"
				checked={mod.pinned}
				onchange={() => void content.setPinned([mod.key], !mod.pinned)}
			/>
		</label>
		<label class="kv">
			<span class="k">Bundle in exports</span>
			<input
				type="checkbox"
				class="switch"
				checked={!mod.noBundle}
				onchange={() => void content.setBundled([mod.key], !!mod.noBundle)}
			/>
		</label>
		<p class="small hint">
			If an export can bundle this mod as an override when unable to find its link.
		</p>
	</section>
	{#if required.length + optional.length + missing.length > 0}
		<section class="inspector-section">
			<div class="band">Dependencies</div>
			{#each [...required, ...optional] as dep (dep.key)}
				<button class="item-row" onclick={() => tabs.open('mod', dep.key)}>
					<ModIcon url={content.meta[dep.projectId]?.iconUrl} size={16} />
					<span class="ellipsis">{dep.name}</span>
					<span class="tag">{required.includes(dep) ? 'required' : 'optional'}</span>
					<span class="spacer"></span>
					<span class="tag added">✓ in pack</span>
				</button>
			{/each}
			{#each missing as dep (dep.projectId)}
				<div class="item-row">
					<ModIcon size={16} />
					<span class="ellipsis">{dep.name}</span>
					<span class="tag">optional</span>
					<span class="spacer"></span>
					<button class="btn small" onclick={pending(() => content.addSuggested(dep))}>Add</button>
				</div>
			{/each}
		</section>
	{/if}
	<ConfigList contentKey={mod.key} />
	{#if instance.bound}
		<section class="inspector-section">
			<div class="band">Instance</div>
			<div class="kv">
				<span class="k ellipsis">{instance.name}</span>
				<span class="v">{instanceWord}</span>
			</div>
			<label class="kv">
				<span class="k">Instance Sync</span>
				<input
					type="checkbox"
					class="switch"
					checked={!syncRule}
					onchange={() =>
						void (syncRule
							? instance.unignore([syncRule.key])
							: instance.ignore([{ key: syncKey, label: mod.name }]))}
				/>
			</label>
		</section>
	{/if}
</div>
<div class="bar footer-bar">
	{#if update}
		<button
			class="btn"
			onclick={pending(() => updatePins([{ key: mod.key, version: update.versionId }]))}
			>Update to {update.versionNumber}</button
		>
	{/if}
	{#if showOpen}
		<button class="btn" onclick={() => tabs.open('mod', mod.key)}
			>Open page <span class="keys">↵</span></button
		>
	{:else}
		<button class="btn danger" onclick={pending(() => removeContent([mod.key]))}>Remove</button>
	{/if}
	<span class="spacer"></span>
	<IconButton
		icon={Ellipsis}
		label="More"
		onclick={(e) => menus.below(e.currentTarget as Element, contentMenu([mod.key]))}
	/>
</div>

<style>
	.words {
		display: flex;
		gap: 8px;
		margin-top: 2px;
	}

	.update {
		color: var(--blue);
	}

	.version :global(svg),
	.update :global(svg) {
		display: inline-block;
		margin-right: 3px;
		vertical-align: -2px;
	}

	.kv :global(.grow) {
		flex: 1;
		min-width: 0;
		height: 26px;
	}

	p.hint {
		padding: 2px 0 6px;
	}

	.linked {
		display: block;
		margin-top: -4px;
		padding: 0 0 4px 98px;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		color: var(--grey);
	}

	.band-detail {
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		font-weight: 400;
	}
</style>

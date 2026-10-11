<script lang="ts">
	import { deleteLocalFile } from '@/lib/flows/content'
	import { content } from '@/lib/state/content.svelte'
	import { ws } from '@/lib/state/workspace.svelte'
	import type { ArchiveDep, LocalContent } from '@/lib/tauri/types'
	import { reveal } from '@/lib/tauri/windows'
	import { bytes, loaderName } from '@/lib/util/format'
	import { join } from '@/lib/util/path'
	import { fileManager } from '@/lib/util/platform'
	import ModIcon from '@/ui/ModIcon.svelte'
	import { pending } from '@/ui/pending'

	import ConfigList from './ConfigList.svelte'

	interface Props {
		file: LocalContent
	}

	let { file }: Props = $props()

	const meta = $derived(file.meta)
	const deps = $derived(
		(meta?.dependencies ?? []).filter((d) => d.kind === 'required' || d.kind === 'optional'),
	)
	const conflicts = $derived(
		(meta?.dependencies ?? []).filter((d) => d.kind === 'incompatible' || d.kind === 'discouraged'),
	)

	function normalize(text: string): string {
		return text.toLowerCase().replace(/[^a-z0-9]/g, '')
	}

	function inPack(dep: ArchiveDep): boolean {
		const id = normalize(dep.id)
		const published = content.mods.some(
			(m) => normalize(m.slug) === id || normalize(m.name) === id || normalize(m.projectId) === id,
		)
		return (
			published ||
			content.unpublished.some(
				(other) =>
					other.relPath !== file.relPath &&
					(other.meta?.ids ?? []).some((otherId) => normalize(otherId) === id),
			)
		)
	}

	function showFile(): void {
		if (ws.dir) void reveal(join(ws.dir, 'overrides', file.relPath))
	}
</script>

<div class="inspector-head">
	<ModIcon url={meta?.icon} size={32} />
	<div class="heading">
		<span class="strong ellipsis">{meta?.name ?? file.filename}</span>
		<span class="small mono ellipsis">{meta?.version ?? ''} · Override</span>
	</div>
</div>
<div class="inspector-body">
	{#if meta?.description}<p class="description">{meta.description}</p>{/if}
	<section class="inspector-section">
		<div class="band">Details</div>
		<div class="kv"><span class="k">File</span><span class="v mono">{file.filename}</span></div>
		<div class="kv"><span class="k">Size</span><span class="v">{bytes(file.size)}</span></div>
		{#if meta?.ids?.length}
			<div class="kv">
				<span class="k">Mod ID</span><span class="v mono">{meta.ids.join(', ')}</span>
			</div>
		{/if}
		{#if meta?.loaders?.length}
			<div class="kv">
				<span class="k">Loader</span><span class="v">{meta.loaders.map(loaderName).join(', ')}</span
				>
			</div>
		{/if}
		{#if meta?.minecraft}
			<div class="kv">
				<span class="k">Minecraft</span><span class="v mono">{meta.minecraft}</span>
			</div>
		{/if}
		{#if meta?.side}
			<div class="kv">
				<span class="k">Side</span><span class="v"
					>{meta.side === 'client' ? 'Client' : 'Server'}</span
				>
			</div>
		{/if}
		{#if meta?.authors?.length}
			<div class="kv">
				<span class="k">Authors</span><span class="v">{meta.authors.join(', ')}</span>
			</div>
		{/if}
	</section>
	{#if deps.length > 0}
		<section class="inspector-section">
			<div class="band">Dependencies</div>
			{#each deps as dep (dep.id)}
				{@const found = inPack(dep)}
				<div class="item-row">
					<span class="mono ellipsis">{dep.id}</span>
					<span class="tag">{dep.kind}</span>
					<span class="spacer"></span>
					{#if found}
						<span class="tag added">✓ in pack</span>
					{:else if dep.kind === 'required'}
						<span class="tag removed">missing</span>
					{:else}
						<span class="tag">not in pack</span>
					{/if}
				</div>
			{/each}
		</section>
	{/if}
	{#if conflicts.length > 0}
		<section class="inspector-section">
			<div class="band">Conflicts</div>
			{#each conflicts as dep (dep.id)}
				{@const found = inPack(dep)}
				<div class="item-row">
					<span class="mono ellipsis">{dep.id}</span>
					<span class="tag">{dep.kind === 'incompatible' ? "can't run with" : 'discouraged'}</span>
					<span class="spacer"></span>
					{#if found}
						<span class="tag {dep.kind === 'incompatible' ? 'removed' : 'changed'}">✗ in pack</span>
					{/if}
				</div>
			{/each}
		</section>
	{/if}
	{#key file.relPath}
		<ConfigList contentKey="local:{file.relPath}" />
	{/key}
</div>
<div class="bar footer-bar">
	<button class="btn" onclick={showFile}>Show in {fileManager}</button>
	<span class="spacer"></span>
	<button class="btn danger" onclick={pending(() => deleteLocalFile(file))}>Delete</button>
</div>

<style>
	.description {
		margin-bottom: 8px;
	}
</style>

<script lang="ts">
	import { discard } from '@/lib/flows/git'
	import { changeWord } from '@/lib/git/changes'
	import { SIDE_WORDS } from '@/lib/instance/items'
	import { app } from '@/lib/state/app.svelte'
	import { content } from '@/lib/state/content.svelte'
	import { files } from '@/lib/state/files.svelte'
	import { git } from '@/lib/state/git.svelte'
	import { instance } from '@/lib/state/instance.svelte'
	import { preserve } from '@/lib/state/preserve.svelte'
	import { tabs, type Doc } from '@/lib/state/tabs.svelte'
	import { ws } from '@/lib/state/workspace.svelte'
	import { reveal } from '@/lib/tauri/windows'
	import { bytes } from '@/lib/util/format'
	import { basename, join } from '@/lib/util/path'
	import { fileManager } from '@/lib/util/platform'
	import { pending } from '@/ui/pending'

	interface Props {
		doc: Doc
	}

	let { doc }: Props = $props()

	const rel = $derived(doc.arg)
	const buffer = $derived(files.buffers[rel])
	const dirty = $derived(files.isDirty(rel))
	const instancePath = $derived(
		rel.startsWith('overrides/') ? rel.slice('overrides/'.length) : null,
	)
	const gitChange = $derived(git.changes.find((c) => c.path === rel))
	const syncKey = $derived(instancePath ? `file:${instancePath}` : null)
	const syncItem = $derived(syncKey ? instance.items.find((i) => i.key === syncKey) : undefined)
	const syncRule = $derived(syncKey ? instance.ignoredEntryFor(syncKey) : null)
	const editRule = $derived(instancePath ? preserve.fileRule(instancePath) : null)
	const editable = $derived(editRule?.unlock ?? false)
	const owner = $derived.by(() => {
		const name = basename(rel).toLowerCase()
		return content.mods.find((m) => {
			const keys = [m.slug, m.projectId]
				.filter((k) => k && k.length > 2)
				.map((k) => k.toLowerCase())
			return keys.some((k) => name.includes(k))
		})
	})
	const gitWord = $derived(gitChange ? changeWord(gitChange) : null)
</script>

<div class="inspector-head">
	<div class="heading">
		<span class="strong ellipsis">{basename(rel)}</span>
		<span class="small mono ellipsis">{rel}</span>
	</div>
</div>
<div class="inspector-body">
	<section class="inspector-section">
		<div class="band">File</div>
		{#if owner}
			<button class="kv" onclick={() => tabs.open('mod', owner.key)}>
				<span class="k">Mod</span><span class="v">{owner.name}</span>
			</button>
		{/if}
		<div class="kv">
			<span class="k">Size</span><span class="v">{bytes(buffer?.size ?? 0)}</span>
		</div>
		<div class="kv">
			<span class="k">State</span>
			<span class="v">
				{#if dirty}unsaved{:else if gitWord}<span class="tag {gitWord}">{gitWord}</span
					>{:else}committed{/if}
			</span>
		</div>
	</section>
	{#if instancePath}
		<section class="inspector-section">
			<div class="band">Updates</div>
			<label class="kv">
				<span class="k">Players can edit</span>
				<input
					type="checkbox"
					class="switch"
					checked={editable}
					onchange={() => instancePath && void preserve.setFile(instancePath, !editable)}
				/>
			</label>
			{#if editRule && editRule.path !== instancePath}
				<div class="kv"><span class="k">By</span><span class="v mono">{editRule.line}</span></div>
			{/if}
		</section>
	{/if}
	{#if instancePath && instance.bound}
		<section class="inspector-section">
			<div class="band">Instance</div>
			<div class="kv">
				<span class="k ellipsis">{instance.name}</span>
				<span class="v"
					>{syncRule ? 'ignored' : syncItem ? SIDE_WORDS[syncItem.side] : 'in sync'}</span
				>
			</div>
			{#if syncRule}
				<div class="kv">
					<span class="k">Ignored</span>
					<span class="v mono ellipsis">{syncRule.label}</span>
					<button class="btn small" onclick={pending(() => instance.unignore([syncRule.key]))}
						>Stop ignoring</button
					>
				</div>
			{:else}
				<label class="kv">
					<input
						type="checkbox"
						class="switch"
						bind:checked={app.prefs.pushOnSave}
						onchange={() => app.save()}
					/>
					<span>Push to {instance.name} on save</span>
				</label>
				<div class="kv">
					<button
						class="btn small"
						disabled={!instancePath}
						onclick={pending(() =>
							instance.ignore([{ key: `file:${instancePath}`, label: instancePath ?? '' }]),
						)}>Ignore for {instance.name}</button
					>
				</div>
			{/if}
		</section>
	{/if}
</div>
<div class="bar footer-bar">
	{#if dirty}
		<button class="btn" onclick={() => files.revert(rel)}>Revert</button>
	{:else if gitChange}
		<button class="btn danger" onclick={pending(() => discard([rel]))}>Discard…</button>
	{/if}
	<span class="spacer"></span>
	<button class="btn quiet" onclick={() => ws.dir && void reveal(join(ws.dir, rel))}
		>Show in {fileManager}</button
	>
</div>

<style>
	button.kv {
		width: 100%;
		text-align: left;
	}

	button.kv:hover .v {
		text-decoration: underline;
	}
</style>

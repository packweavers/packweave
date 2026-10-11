<script lang="ts">
	import { coversKey, ignoreEntry, SIDE_WORDS } from '@/lib/instance/items'
	import { instance } from '@/lib/state/instance.svelte'
	import { selection } from '@/lib/state/selection.svelte'
	import type { Doc } from '@/lib/state/tabs.svelte'
	import { tasks } from '@/lib/state/tasks.svelte'
	import { ws } from '@/lib/state/workspace.svelte'
	import { syncFileDiff } from '@/lib/tauri/instance'
	import { ago } from '@/lib/util/format'
	import DiffView from '@/ui/DiffView.svelte'
	import Empty from '@/ui/Empty.svelte'
	import { pending } from '@/ui/pending'

	interface Props {
		doc: Doc
	}

	let { doc }: Props = $props()

	const key = $derived(selection.of(doc.id)[0] ?? null)
	const item = $derived(instance.items.find((i) => i.key === key) ?? null)
	const entry = $derived(key ? (instance.ignored.find((e) => e.key === key) ?? null) : null)
	const rule = $derived(item?.ignored ? instance.ignoredEntryFor(item.key) : null)
	const covered = $derived(
		entry ? instance.items.filter((i) => i.ignored && coversKey(entry, i.key)) : [],
	)
	const path = $derived(item?.kind === 'file' ? item.path : null)
	const rawKind = $derived(item?.rawKind ?? '')
	const where = $derived.by(() => {
		if (item?.side === 'pull') return `in ${instance.name}`
		if (item?.side === 'push') return `in ${ws.name}`
		return 'on both sides'
	})

	let diff = $state<{ path: string; text: string } | null>(null)
	const shownDiff = $derived(diff && diff.path === path ? diff.text : null)

	$effect(() => {
		const dir = ws.dir
		const file = path
		const kind = rawKind
		void instance.report
		if (!dir || !file) return
		void syncFileDiff(dir, file, kind)
			.then((text) => (diff = { path: file, text }))
			.catch((error) => tasks.fail('Reading the diff', error))
	})

	function packState(kind: string): string {
		if (kind === 'instance_only' || kind === 'unknown') return 'not in pack'
		return kind === 'disabled' ? 'disabled in pack' : 'in pack'
	}
</script>

{#if item}
	<div class="inspector-head">
		<div class="heading">
			<span class="strong ellipsis">{item.label}</span>
			<span class="small mono ellipsis">{item.path ?? item.detail}</span>
			<span class="words"
				><span class="tag {item.change}">{item.change}</span><span class="tag"
					>{SIDE_WORDS[item.side]}</span
				>{#if item.ignored}<span class="tag">ignored</span>{/if}</span
			>
		</div>
	</div>
	<div class="inspector-body">
		<section class="inspector-section">
			<div class="band">Details</div>
			<div class="kv">
				<span class="k">Changed</span><span class="v"
					>{where}{item.modified ? ` · ${ago(item.modified * 1000)}` : ''}</span
				>
			</div>
			{#if item.kind === 'mod'}
				<div class="kv">
					<span class="k">Pack</span>
					<span class="v">{packState(item.rawKind)}</span>
				</div>
			{/if}
		</section>
		{#if shownDiff !== null}
			<section class="inspector-section">
				<div class="band">Difference</div>
				<div class="diff"><DiffView diff={shownDiff} /></div>
			</section>
		{/if}
	</div>
	<div class="bar footer-bar">
		{#if rule}
			<button class="btn" onclick={pending(() => instance.unignore([rule.key]))}
				>{rule.key === item.key ? 'Stop ignoring' : `Stop ignoring ${rule.label}`}</button
			>
		{:else}
			{#if item.side !== 'pull'}<button class="btn" onclick={pending(() => instance.push([item]))}
					>Push</button
				>{/if}
			{#if item.side !== 'push'}<button class="btn" onclick={pending(() => instance.pull([item]))}
					>Pull</button
				>{/if}
			{#if item.side !== 'both'}<button class="btn" onclick={pending(() => instance.revert([item]))}
					>Revert</button
				>{/if}
			<span class="spacer"></span>
			<button class="btn" onclick={pending(() => instance.ignore([ignoreEntry(item)]))}
				>Ignore</button
			>
		{/if}
	</div>
{:else if entry}
	<div class="inspector-head">
		<div class="heading">
			<span class="strong ellipsis" class:mono={entry.key.startsWith('file:')}>{entry.label}</span>
			<span class="small">Ignored for {instance.name}</span>
		</div>
	</div>
	<div class="inspector-body">
		<section class="inspector-section">
			<div class="band">Differences</div>
			{#each covered as change (change.key)}
				<div class="item-row">
					<span class="ellipsis mono">{change.path ?? change.label}</span>
					<span class="spacer"></span>
					<span class="tag {change.change}">{change.change}</span>
					<span class="small">{SIDE_WORDS[change.side]}</span>
				</div>
			{:else}
				<span class="small">Same on both sides</span>
			{/each}
		</section>
	</div>
	<div class="bar footer-bar">
		<button class="btn" onclick={pending(() => instance.unignore([entry.key]))}
			>Stop ignoring</button
		>
	</div>
{:else}
	<div class="inspector-head">
		<div class="heading">
			<span class="strong">{instance.name || 'Instance'}</span>
			<span class="small">{instance.binding?.launcher ?? ''}</span>
		</div>
	</div>
	<div class="inspector-body">
		{#if instance.binding}
			<section class="inspector-section">
				<div class="kv">
					<span class="k">Game</span><span class="v"
						>{instance.binding.minecraft ?? '?'} · {instance.binding.loader ?? 'vanilla'}</span
					>
				</div>
				<div class="kv">
					<span class="k">To push</span><span class="v">{instance.toPush.length}</span>
				</div>
				<div class="kv">
					<span class="k">To pull</span><span class="v">{instance.toPull.length}</span>
				</div>
				<div class="kv">
					<span class="k">Ignored</span><span class="v">{instance.ignored.length}</span>
				</div>
			</section>
		{:else}
			<Empty title="No instance" />
		{/if}
	</div>
{/if}

<style>
	.words {
		display: flex;
		gap: 8px;
	}

	.diff {
		margin: 0 -14px;
		overflow-x: auto;
	}
</style>

<script lang="ts">
	import { contentPathType, versionLoader } from '@/lib/content/types'
	import { abortMerge, commitMerge } from '@/lib/flows/branches'
	import { withGit } from '@/lib/flows/git'
	import { displayPath } from '@/lib/git/changes'
	import { git } from '@/lib/state/git.svelte'
	import { tabs, type Doc } from '@/lib/state/tabs.svelte'
	import { tasks } from '@/lib/state/tasks.svelte'
	import { toasts } from '@/lib/state/toasts.svelte'
	import { ws } from '@/lib/state/workspace.svelte'
	import { versions as loadVersions } from '@/lib/tauri/catalog'
	import { read } from '@/lib/tauri/files'
	import {
		conflictSides,
		resolveConflict,
		resolveContentVersion,
		resolveWith,
	} from '@/lib/tauri/git'
	import type { ConflictSides, ContentItem, VersionInfo } from '@/lib/tauri/types'
	import { plural } from '@/lib/util/format'
	import Empty from '@/ui/Empty.svelte'
	import { pending } from '@/ui/pending'
	import Select from '@/ui/Select.svelte'

	interface Props {
		doc: Doc
	}

	let { doc }: Props = $props()

	const into = $derived(git.branch ?? 'this branch')
	const from = $derived(git.merge?.note?.from ?? 'the other branch')

	let sides = $state<Record<string, ConflictSides>>({})
	let choices = $state<Record<string, VersionInfo[]>>({})
	let picked = $state<Record<string, string>>({})
	const requested = new Set<string>()

	$effect(() => {
		const dir = ws.dir
		if (!dir) return
		for (const path of git.conflicts) {
			if (requested.has(path)) continue
			requested.add(path)
			void conflictSides(dir, path)
				.then((found) => (sides[path] = found))
				.catch((error) => {
					requested.delete(path)
					tasks.fail(`Reading both sides of ${path}`, error)
				})
		}
	})

	function version(item: ContentItem | null): string {
		if (!item) return 'removed'
		return (
			item.sources[item.preferred]?.versionNumber ?? item.sources[item.preferred]?.versionId ?? '?'
		)
	}

	async function settle(path: string, work: (dir: string) => Promise<void>): Promise<void> {
		const dir = ws.dir
		if (!dir) return
		await withGit(async () => {
			await work(dir)
			delete sides[path]
			requested.delete(path)
			await git.refresh()
		})
	}

	async function loadChoices(path: string, item: ContentItem): Promise<void> {
		const manifest = ws.manifest
		if (!manifest || choices[path]) return
		const id = item.sources[item.preferred]?.id
		if (!id) return
		choices[path] = await loadVersions(
			id,
			item.preferred,
			manifest.minecraft,
			versionLoader(contentPathType(path), manifest.loader),
		).catch((error) => {
			toasts.error(error)
			return []
		})
	}

	async function markResolved(path: string): Promise<void> {
		const dir = ws.dir
		if (!dir) return
		const file = await read(dir, path)
		if (file.text && /^(<{7}|={7}|>{7})/m.test(file.text)) {
			tabs.open('file', path)
			return
		}
		await settle(path, (d) => resolveWith(d, path, file.text ?? ''))
	}
</script>

<div class="bar doc-toolbar">
	<span class="doc-title">Merging {from} into {into}</span>
	{#if git.conflicts.length > 0}<span
			>· {plural(git.conflicts.length, 'conflict', 'conflicts')} left</span
		>{/if}
	<span class="spacer"></span>
	{#if git.merging}<button class="btn danger" onclick={pending(() => abortMerge())}>Abort…</button
		>{/if}
</div>

{#if !git.merging}
	<Empty title="No merge in progress">
		<button class="btn" onclick={() => tabs.close(doc.id)}>Close</button>
	</Empty>
{:else if git.conflicts.length === 0}
	<Empty title="No conflicts left">
		<button class="btn primary" onclick={pending(() => commitMerge())}>Commit merge</button>
	</Empty>
{:else}
	<div class="scroll cards">
		{#each git.conflicts as path (path)}
			{@const side = sides[path]}
			<div class="card">
				{#if path.startsWith('content/') && side}
					{@const ours = side.oursItem}
					{@const theirs = side.theirsItem}
					<div class="bar band">{ours?.name ?? theirs?.name ?? displayPath(path)} version</div>
					<div class="trow">
						<span class="who mono">{into}</span>
						<span class="mono">{version(ours)}</span>
						<span class="spacer"></span>
						<button
							class="btn"
							onclick={pending(() => settle(path, (d) => resolveConflict(d, path, 'ours')))}
							>Keep</button
						>
					</div>
					<div class="trow">
						<span class="who mono">{from}</span>
						<span class="mono">{version(theirs)}</span>
						<span class="spacer"></span>
						<button
							class="btn"
							onclick={pending(() => settle(path, (d) => resolveConflict(d, path, 'theirs')))}
							>Take other</button
						>
					</div>
					{#if ours ?? theirs}
						{@const item = (ours ?? theirs)!}
						<div class="trow">
							<span class="who">Any version</span>
							{#if choices[path]}
								<Select
									options={choices[path].map((v) => ({ value: v.id, label: v.versionNumber }))}
									bind:value={picked[path]}
									mono
									class="pick"
								/>
								<span class="spacer"></span>
								<button
									class="btn"
									disabled={!picked[path]}
									onclick={pending(() =>
										settle(path, (d) => resolveContentVersion(d, path, picked[path])),
									)}
								>
									Use
								</button>
							{:else}
								<span class="spacer"></span>
								<button class="btn" onclick={pending(() => loadChoices(path, item))}>Choose…</button
								>
							{/if}
						</div>
					{/if}
				{:else}
					<div class="bar band">
						<span class="mono">{displayPath(path)}</span>
						<span class="spacer"></span>
						<button
							class="btn"
							onclick={pending(() => settle(path, (d) => resolveConflict(d, path, 'ours')))}
							>Keep {into}</button
						>
						<button
							class="btn"
							onclick={pending(() => settle(path, (d) => resolveConflict(d, path, 'theirs')))}
							>Keep {from}</button
						>
						<button class="btn" onclick={() => tabs.open('file', path)}>Edit</button>
						<button class="btn" onclick={pending(() => markResolved(path))}>Mark settled</button>
					</div>
					{#if side}
						<div class="sides">
							<div class="side">
								<div class="side-title small mono">{into}</div>
								<pre class="code">{side.ours ?? '(deleted)'}</pre>
							</div>
							<div class="side">
								<div class="side-title small mono">{from}</div>
								<pre class="code">{side.theirs ?? '(deleted)'}</pre>
							</div>
						</div>
					{/if}
				{/if}
			</div>
		{/each}
	</div>
{/if}

<style>
	.cards {
		padding: 16px;
		display: flex;
		flex-direction: column;
		gap: 16px;
	}

	.card .band {
		min-height: var(--bar);
		padding-left: 12px;
	}

	.who {
		flex: 0 0 160px;
	}

	.card :global(.pick) {
		width: 200px;
	}

	.sides {
		display: flex;
		max-height: 340px;
	}

	.side {
		flex: 1 1 50%;
		min-width: 0;
		overflow: auto;
	}

	.side + .side {
		border-left: 1px solid var(--line);
	}

	.side-title {
		padding: 4px 12px;
		border-bottom: 1px solid var(--line);
	}

	.side pre {
		margin: 0;
		padding: 8px 12px;
		user-select: text;
		-webkit-user-select: text;
	}
</style>

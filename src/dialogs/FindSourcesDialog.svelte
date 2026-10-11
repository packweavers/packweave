<script lang="ts">
	import { onMount } from 'svelte'

	import type { SourceMatch } from '@/lib/flows/content'
	import { content } from '@/lib/state/content.svelte'
	import { ws } from '@/lib/state/workspace.svelte'
	import { errorText } from '@/lib/tauri/call'
	import { convertLookup, search } from '@/lib/tauri/catalog'
	import type { CatalogHit, ConvertCandidate, ProviderId } from '@/lib/tauri/types'
	import { compactCount, plural, providerName } from '@/lib/util/format'
	import Author from '@/ui/Author.svelte'
	import Dialog from '@/ui/Dialog.svelte'
	import { focusOnMount } from '@/ui/focus'
	import ModIcon from '@/ui/ModIcon.svelte'

	interface Props {
		target: ProviderId
		matches: SourceMatch[]
		close: (result?: { key: string; provider: ProviderId; id: string }[]) => void
	}

	let { target, matches, close }: Props = $props()

	let skipped = $state<string[]>([])
	let picked = $state<Record<string, ConvertCandidate>>({})
	let editing = $state<string | null>(null)
	let query = $state('')
	let looking = $state(false)
	let failure = $state('')
	let hits = $state<CatalogHit[]>([])
	let searching = $state(false)
	let timer: ReturnType<typeof setTimeout> | null = null
	let asked = 0

	const rows = $derived(matches.map((m) => ({ key: m.key, match: picked[m.key] ?? m.match })))
	const found = $derived(rows.filter((m) => m.match !== null))
	const missing = $derived(rows.filter((m) => m.match === null))
	const chosen = $derived(found.filter((m) => !skipped.includes(m.key)))
	const platform = $derived(providerName(target))

	function nameOf(key: string): string {
		return content.find(key)?.name ?? key
	}

	function edit(key: string): void {
		editing = key
		query = content.find(key)?.name ?? ''
		failure = ''
		hits = []
		void find()
	}

	function looksLikeLink(text: string): boolean {
		return /^https?:\/\//i.test(text) || /^\d+$/.test(text)
	}

	function typed(): void {
		failure = ''
		if (timer) clearTimeout(timer)
		timer = setTimeout(() => void find(), 250)
	}

	async function find(): Promise<void> {
		const text = query.trim()
		const key = editing
		const manifest = ws.manifest
		const ask = ++asked
		if (!key || !manifest || text.length < 2 || looksLikeLink(text)) {
			hits = []
			return
		}
		searching = true
		try {
			const found = await search({
				provider: target,
				query: text,
				minecraft: manifest.minecraft,
				loader: manifest.loader,
				projectType: content.find(key)?.projectType ?? 'mod',
				offset: 0,
				limit: 6,
			})
			if (ask === asked) hits = found
		} catch (error) {
			if (ask !== asked) return
			hits = []
			failure = errorText(error)
		} finally {
			if (ask === asked) searching = false
		}
	}

	async function choose(reference: string): Promise<void> {
		const key = editing
		const dir = ws.dir
		if (!key || !dir || !reference.trim() || looking) return
		looking = true
		failure = ''
		try {
			const match = await convertLookup(dir, key, target, reference.trim())
			picked = { ...picked, [key]: match }
			skipped = skipped.filter((k) => k !== key)
			editing = null
			hits = []
		} catch (error) {
			failure = errorText(error)
		} finally {
			looking = false
		}
	}

	function submit(): void {
		const text = query.trim()
		if (!text) return
		if (!looksLikeLink(text) && hits.length > 0) {
			void choose(hits[0].slug || hits[0].projectId)
			return
		}
		void choose(text)
	}

	onMount(() => {
		if (matches.length === 1 && !matches[0].match) edit(matches[0].key)
	})

	function confirm(): void {
		close(chosen.map((m) => ({ key: m.key, provider: target, id: m.match!.id })))
	}
</script>

{#snippet lookup(key: string)}
	{#if editing === key}
		<form
			class="lookup"
			onsubmit={(e) => {
				e.preventDefault()
				submit()
			}}
		>
			<input
				class="field"
				bind:value={query}
				placeholder="Search {platform} or paste a link"
				spellcheck="false"
				use:focusOnMount={true}
				oninput={typed}
				onkeydown={(e) => {
					if (e.key === 'Escape') {
						e.preventDefault()
						e.stopPropagation()
						editing = null
					}
				}}
			/>
			<button class="btn" type="button" onclick={() => (editing = null)}>Cancel</button>
		</form>
		{#if failure}<p class="small failure">{failure}</p>{/if}
		{#if hits.length > 0 || searching}
			<div class="hits" class:busy={searching || looking}>
				{#each hits as hit (hit.projectId)}
					<button
						class="hit"
						type="button"
						disabled={looking}
						onclick={() => void choose(hit.slug || hit.projectId)}
					>
						<ModIcon url={hit.iconUrl} size={20} />
						<span class="hit-text">
							<span class="ellipsis">{hit.title}</span>
							<span class="small hint ellipsis">by {hit.author}</span>
						</span>
						<span class="mono small hint">{compactCount(hit.downloads)}</span>
					</button>
				{/each}
				{#if hits.length === 0}<span class="small hint searching">Searching…</span>{/if}
			</div>
		{/if}
	{/if}
{/snippet}

<Dialog title="Find on {platform}" width={560} onclose={() => close()}>
	<p class="small hint">
		{#if matches.length === 1}
			{found.length ? 'Check that this is the same mod.' : `Not found on ${platform} by name.`}
		{:else}
			Found {found.length} of {matches.length}. Check that each match is the same mod.
		{/if}
	</p>
	<div class="list">
		{#each found as row (row.key)}
			<div class="item">
				<label class="row">
					<input
						type="checkbox"
						checked={!skipped.includes(row.key)}
						onchange={(e) =>
							(skipped = e.currentTarget.checked
								? skipped.filter((k) => k !== row.key)
								: [...skipped, row.key])}
					/>
					<ModIcon url={row.match?.iconUrl} size={32} />
					<span class="text">
						<span class="name">{nameOf(row.key)}</span>
						<span class="small match"
							><span class="ellipsis">{row.match?.name}</span>{#if row.match?.author}<Author
									name={row.match.author}
									icon={row.match.authorIconUrl}
								/>{/if}</span
						>
					</span>
					{#if row.match?.version}<span class="mono small">{row.match.version}</span>{/if}
					{#if editing !== row.key}
						<button
							class="btn quiet"
							onclick={(e) => {
								e.preventDefault()
								edit(row.key)
							}}>Change…</button
						>
					{/if}
				</label>
				{@render lookup(row.key)}
			</div>
		{/each}
		{#each missing as row (row.key)}
			<div class="item">
				<div class="row missing">
					<span class="name">{nameOf(row.key)}</span>
					<span class="spacer"></span>
					<span class="tag removed">not found</span>
					{#if editing !== row.key}
						<button class="btn quiet" onclick={() => edit(row.key)}>Search…</button>
					{/if}
				</div>
				{@render lookup(row.key)}
			</div>
		{/each}
	</div>

	{#snippet footer()}
		<button class="btn" onclick={() => close()}>Cancel</button>
		<button class="btn primary" data-default disabled={chosen.length === 0} onclick={confirm}
			>Add {plural(chosen.length, 'source', 'sources')}</button
		>
	{/snippet}
</Dialog>

<style>
	.hint {
		margin-bottom: 8px;
	}

	.list {
		max-height: 420px;
		overflow: auto;
		border: 1px solid var(--line);
	}

	.item + .item {
		border-top: 1px solid var(--line);
	}

	.row {
		display: flex;
		align-items: center;
		gap: 10px;
		min-height: 44px;
		padding: 6px 12px;
	}

	.text {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
	}

	.name {
		font-weight: 500;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.match {
		min-width: 0;
		display: flex;
		align-items: center;
		gap: 8px;
		color: var(--green);
	}

	.match :global(.author) {
		color: var(--grey);
	}

	.missing .name {
		color: var(--grey);
	}

	.lookup {
		display: flex;
		gap: 6px;
		padding: 0 12px 10px 37px;
	}

	.lookup .field {
		flex: 1;
		min-width: 0;
	}

	.failure {
		padding: 0 12px 10px 37px;
		color: var(--red);
	}

	.hits {
		display: flex;
		flex-direction: column;
		margin: 0 12px 10px 37px;
		border: 1px solid var(--line);
	}

	.hits.busy {
		opacity: 0.6;
	}

	.hit {
		display: flex;
		align-items: center;
		gap: 10px;
		min-height: 36px;
		padding: 4px 10px;
		text-align: left;
	}

	.hit + .hit {
		border-top: 1px solid var(--line);
	}

	.hit:hover:not(:disabled) {
		background: var(--hover);
	}

	.hit-text {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
	}

	.searching {
		padding: 8px 10px;
	}
</style>

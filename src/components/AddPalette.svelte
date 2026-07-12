<script lang="ts">
	import { Search, Plus, Check, ExternalLink, Link, ClipboardList, TriangleAlert } from '@lucide/svelte'
	import Avatar from './ui/Avatar.svelte'
	import ButtonStyled from './ui/ButtonStyled.svelte'
	import { api, openModrinthPage, openCurseforgePage } from '../api'
	import { store } from '../lib/store.svelte'
	import { contextMenu } from '../lib/contextmenu.svelte'
	import { tooltip } from '../lib/tooltip'
	import type { BulkLookup, ProjectType, SearchHit } from '../types'
	import { formatCount } from '../util'

	let { onclose }: { onclose?: () => void } = $props()

	const sources = $derived([
		...store.enabledProviders.map((p) => ({ id: p.id, label: p.displayName })),
		{ id: 'url', label: 'URL' },
	])

	const TABS: { type: ProjectType; label: string }[] = [
		{ type: 'mod', label: 'Mods' },
		{ type: 'resourcepack', label: 'Resource packs' },
		{ type: 'shader', label: 'Shaders' },
	]

	let source = $state<string>('modrinth')
	let query = $state('')
	let projectType = $state<ProjectType>('mod')
	let results = $state<SearchHit[]>([])
	let loading = $state(false)
	let error = $state<string | null>(null)
	let urlValue = $state('')
	let urlName = $state('')
	let reqId = 0

	const sourceLabel = $derived(store.providerName(source))
	const canOpen = $derived(source === 'modrinth' || source === 'curseforge')

	async function runSearch() {
		if (source === 'url') {
			results = []
			return
		}
		const id = ++reqId
		loading = true
		error = null
		try {
			const hits = await api.search(
				source,
				query,
				store.minecraft,
				store.loader,
				projectType,
				0,
				40,
			)
			if (id === reqId) results = hits
		} catch (e) {
			if (id === reqId) {
				error = `${e}`
				results = []
			}
		} finally {
			if (id === reqId) loading = false
		}
	}

	function addHit(hit: SearchHit) {
		store.addContent(source, {
			projectId: hit.project_id,
			slug: hit.slug,
			name: hit.title,
			projectType: hit.project_type,
		})
	}

	function openHit(hit: SearchHit) {
		if (source === 'curseforge') openCurseforgePage(hit.slug, hit.project_type)
		else if (source === 'modrinth') openModrinthPage(hit.slug)
	}

	function addUrl() {
		const u = urlValue.trim()
		if (!u) return
		const guessed = u.split('/').pop()?.replace(/\.[^.]+$/, '') || 'file'
		store.addContent('url', {
			projectId: '',
			slug: '',
			name: urlName.trim() || guessed,
			projectType,
			url: u,
		})
		urlValue = ''
		urlName = ''
	}

	let debounce: ReturnType<typeof setTimeout> | undefined
	$effect(() => {
		query
		projectType
		source
		store.minecraft
		store.loader
		clearTimeout(debounce)
		debounce = setTimeout(runSearch, 260)
		return () => clearTimeout(debounce)
	})

	let mode = $state<'search' | 'bulk'>('search')
	let bulkText = $state('')
	let bulkResult = $state<BulkLookup | null>(null)
	let bulkSelected = $state(new Set<string>())
	let bulkBusy = $state(false)

	async function bulkFind() {
		if (!store.pack || !bulkText.trim()) return
		bulkBusy = true
		try {
			bulkResult = await api.bulkLookup(store.pack.dir, bulkText)
			bulkSelected = new Set(bulkResult.found.map((c) => c.projectId))
		} catch (e) {
			store.notify('error', `${e}`)
		} finally {
			bulkBusy = false
		}
	}
	function toggleBulkPick(id: string) {
		if (bulkSelected.has(id)) bulkSelected.delete(id)
		else bulkSelected.add(id)
		bulkSelected = new Set(bulkSelected)
	}
	async function bulkAddSelected() {
		const items = (bulkResult?.found ?? []).filter((c) => bulkSelected.has(c.projectId))
		if (!items.length) return
		await store.bulkAdd(items)
		onclose?.()
	}
</script>

<svelte:window onkeydown={(e) => e.key === 'Escape' && onclose?.()} />

<div
	class="fixed inset-0 z-[65] flex justify-center items-start pt-[10vh] bg-[rgba(6,8,12,0.4)] backdrop-blur-[2px]"
	role="presentation"
	onclick={(e) => {
		if (e.target === e.currentTarget) onclose?.()
	}}
>
	<div class="w-[660px] max-w-[calc(100vw-2rem)] max-h-[74vh] flex flex-col bg-bg-super-raised border border-divider rounded-lg shadow-floating overflow-hidden">
		<div class="flex items-center gap-[0.6rem] px-4 py-[0.85rem] border-b border-divider text-secondary">
			{#if mode === 'bulk'}
				<ClipboardList size={18} />
				<span class="flex-1 text-contrast text-[0.95rem]">Paste a list of links to add at once</span>
			{:else}
				{#if source !== 'url'}
					<Search size={18} />
				{:else}
					<Link size={18} />
				{/if}
				<!-- svelte-ignore a11y_autofocus -->
				{#if source !== 'url'}
					<input
						bind:value={query}
						class="flex-1 bg-transparent border-none text-contrast text-[0.95rem] outline-none"
						placeholder={`Add to your pack through ${sourceLabel}…`}
						autofocus
						spellcheck="false"
					/>
				{:else}
					<input
						bind:value={urlValue}
						class="flex-1 bg-transparent border-none text-contrast text-[0.95rem] outline-none"
						placeholder="Paste a direct download URL (.jar / .zip)…"
						autofocus
						spellcheck="false"
						onkeydown={(e) => e.key === 'Enter' && addUrl()}
					/>
				{/if}
			{/if}
			<kbd
				class="text-[0.66rem] bg-button-bg border border-divider rounded-sm px-[0.35rem] py-[0.1rem] text-secondary"
				>esc</kbd
			>
		</div>

		<div class="flex items-center gap-[0.3rem] px-3 py-2 border-b border-divider">
			{#if mode === 'bulk'}
				<button
					class="inline-flex items-center gap-1 border-none bg-transparent text-secondary text-[0.78rem] px-[0.5rem] py-[0.3rem] rounded-max cursor-pointer hover:text-body hover:bg-button-bg"
					onclick={() => (mode = 'search')}
				>
					← Search
				</button>
			{:else}
				{#each sources as s (s.id)}
					<button
						class="border-none font-semibold text-[0.78rem] px-[0.6rem] py-[0.3rem] rounded-max cursor-pointer {source ===
						s.id
							? 'text-contrast bg-button-bg shadow-[inset_0_0_0_1px_var(--color-divider-dark)]'
							: 'bg-transparent text-secondary hover:text-body hover:bg-button-bg'}"
						onclick={() => (source = s.id)}
					>
						{s.label}
					</button>
				{/each}
				<span class="w-px self-stretch my-[0.15rem] mx-[0.35rem] bg-divider"></span>
				{#each TABS as t (t.type)}
					<button
						class="border-none font-[550] text-[0.78rem] px-[0.6rem] py-[0.3rem] rounded-max cursor-pointer {projectType ===
						t.type
							? 'text-on-brand bg-brand'
							: 'bg-transparent text-secondary hover:text-body hover:bg-button-bg'}"
						onclick={() => (projectType = t.type)}
					>
						{t.label}
					</button>
				{/each}
				<button
					class="inline-flex items-center gap-1 border-none bg-transparent text-secondary text-[0.74rem] px-[0.5rem] py-[0.3rem] rounded-max cursor-pointer hover:text-body hover:bg-button-bg ml-2"
					onclick={() => (mode = 'bulk')}
				>
					<ClipboardList size={13} /> Paste a list
				</button>
			{/if}
			<span class="ml-auto text-[0.72rem] text-secondary">{store.minecraft} · {store.loader}</span>
		</div>

		{#if mode === 'bulk'}
			<div class="flex flex-col min-h-0 overflow-hidden">
				<div class="p-[0.7rem] flex flex-col gap-2">
					<textarea
						bind:value={bulkText}
						rows="5"
						placeholder="Paste Modrinth / CurseForge links (or slugs, one per line). Surrounding text is fine. It finds every link."
						class="w-full bg-bg-inset border border-divider text-contrast rounded-md px-[0.65rem] py-2 text-[0.84rem] outline-none focus:border-brand resize-y font-mono leading-[1.5]"
						spellcheck="false"
					></textarea>
					<div class="flex items-center gap-[0.6rem]">
						<ButtonStyled
							color="brand"
							size="small"
							disabled={!bulkText.trim() || bulkBusy}
							onclick={bulkFind}
						>
							{bulkBusy ? 'Finding…' : 'Find'}
						</ButtonStyled>
						{#if bulkResult}
							<span class="text-[0.74rem] text-secondary">
								{bulkResult.found.length} found{bulkResult.failed.length
									? ` · ${bulkResult.failed.length} unrecognized`
									: ''}
							</span>
						{/if}
					</div>
				</div>
				{#if bulkResult}
					<div class="overflow-y-auto px-[0.5rem] pb-2 flex-1 min-h-0">
						{#each bulkResult.found as c (c.projectId)}
							<label
								class="flex items-center gap-[0.6rem] px-[0.55rem] py-[0.45rem] rounded-md cursor-pointer hover:bg-button-bg"
							>
								<input
									type="checkbox"
									class="w-4 h-4 accent-brand cursor-pointer shrink-0"
									checked={bulkSelected.has(c.projectId)}
									onchange={() => toggleBulkPick(c.projectId)}
								/>
								<Avatar src={c.iconUrl ?? null} alt={c.name} size={30} />
								<div class="flex-1 min-w-0">
									<div
										class="text-[0.84rem] text-contrast whitespace-nowrap overflow-hidden text-ellipsis"
									>
										{c.name}
									</div>
									<div class="text-[0.7rem] text-secondary">
										{store.providerName(c.provider)} · {c.projectType}
									</div>
								</div>
							</label>
						{/each}
						{#if bulkResult.failed.length}
							<div class="mt-2 border-t border-divider pt-2">
								{#each bulkResult.failed as f (f.raw)}
									<div class="flex items-start gap-[0.45rem] px-[0.55rem] py-[0.3rem] text-[0.74rem] text-orange">
										<TriangleAlert size={13} class="shrink-0 mt-[0.1rem]" />
										<span class="min-w-0"
											><span class="font-mono break-all">{f.raw}</span>: {f.reason}</span
										>
									</div>
								{/each}
							</div>
						{/if}
					</div>
					<div
						class="flex items-center justify-end gap-2 px-[0.7rem] py-[0.6rem] border-t border-divider"
					>
						<ButtonStyled
							color="brand"
							disabled={store.busy || bulkSelected.size === 0}
							onclick={bulkAddSelected}
						>
							<Plus size={15} /> Add {bulkSelected.size}
						</ButtonStyled>
					</div>
				{/if}
			</div>
		{:else if source === 'url'}
			<div class="p-[0.85rem] flex flex-col gap-[0.6rem]">
				<input
					bind:value={urlName}
					class="bg-bg-inset border border-divider text-contrast rounded-md px-[0.65rem] py-2 text-[0.85rem] outline-none"
					placeholder="Display name (optional)"
					spellcheck="false"
				/>
				<div class="self-start">
					<ButtonStyled color="brand" disabled={!urlValue.trim() || store.busy} onclick={addUrl}>
						<Plus size={15} /> Add file
					</ButtonStyled>
				</div>
				<p class="m-0 text-[0.74rem] text-secondary leading-[1.5]">
					Bundled as an override. Most platforms reject these, so prefer Modrinth or CurseForge.
				</p>
			</div>
		{:else}
			<div class="overflow-y-auto p-[0.4rem]">
				{#if error}
					<div class="text-center text-red py-[1.8rem] px-4 text-[0.85rem]">{error}</div>
				{:else if loading && !results.length}
					<div class="text-center text-secondary py-[1.8rem] px-4 text-[0.85rem]">Searching…</div>
				{:else if !results.length}
					<div class="text-center text-secondary py-[1.8rem] px-4 text-[0.85rem]">Nothing matches this version and loader.</div>
				{/if}

				{#each results as hit (hit.project_id)}
					<button
						class="flex gap-[0.7rem] w-full text-left bg-transparent border-none px-[0.65rem] py-[0.6rem] rounded-md items-start cursor-pointer disabled:cursor-default {store.busy ||
						store.presentIds.has(hit.project_id)
							? ''
							: 'hover:bg-button-bg'}"
						disabled={store.busy || store.presentIds.has(hit.project_id)}
						onclick={() => addHit(hit)}
						use:contextMenu={() => [
							{
								label: store.presentIds.has(hit.project_id) ? 'Added' : 'Add',
								icon: store.presentIds.has(hit.project_id) ? Check : Plus,
								disabled: store.busy || store.presentIds.has(hit.project_id),
								onSelect: () => addHit(hit),
							},
						]}
					>
						<Avatar src={hit.icon_url} alt={hit.title} size={38} />
						<div class="flex-1 min-w-0">
							<div class="flex items-baseline gap-2">
								<span class="font-semibold text-contrast">{hit.title}</span>
								<span class="text-[0.7rem] text-secondary">by {hit.author}</span>
								{#if canOpen}
									<span
										class="text-secondary cursor-pointer hover:text-contrast"
										role="button"
										tabindex="0"
										use:tooltip={`Open on ${sourceLabel}`}
										onclick={(e) => {
											e.stopPropagation()
											openHit(hit)
										}}
										onkeydown={(e) => {
											if (e.key === 'Enter') {
												e.stopPropagation()
												openHit(hit)
											}
										}}
									>
										<ExternalLink size={12} />
									</span>
								{/if}
							</div>
							<div class="text-body text-[0.78rem] mt-[0.15rem] mb-[0.35rem] overflow-hidden [display:-webkit-box] [-webkit-line-clamp:2] [line-clamp:2] [-webkit-box-orient:vertical]">{hit.description}</div>
							<div class="flex items-center gap-2 flex-wrap text-[0.68rem] text-secondary">
								<span>↓ {formatCount(hit.downloads)}</span>
								{#each hit.display_categories.slice(0, 3) as c (c)}
									<span class="bg-button-bg px-[0.4rem] py-[0.1rem] rounded-sm capitalize">{c}</span>
								{/each}
							</div>
						</div>
						{#if store.presentIds.has(hit.project_id)}
							<span class="inline-flex items-center gap-[0.25rem] text-[0.74rem] text-green shrink-0 self-center"><Check size={14} />Added</span>
						{:else}
							<span class="grid place-items-center w-[1.9rem] h-[1.9rem] rounded-sm bg-brand text-on-brand shrink-0"><Plus size={15} /></span>
						{/if}
					</button>
				{/each}
			</div>
		{/if}
	</div>
</div>

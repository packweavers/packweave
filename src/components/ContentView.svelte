<script lang="ts">
	import { Boxes, ChevronDown, EyeOff, RefreshCw, Search, X } from '@lucide/svelte'
	import Admonition from './ui/Admonition.svelte'
	import ButtonStyled from './ui/ButtonStyled.svelte'
	import ContentFilters from './content/ContentFilters.svelte'
	import ModRow from './content/ModRow.svelte'
	import DepRow from './content/DepRow.svelte'
	import SourceModals from './content/SourceModals.svelte'
	import UpdatesModal from './content/UpdatesModal.svelte'
	import UnpublishedSection from './content/UnpublishedSection.svelte'
	import FeatureTip from './ui/FeatureTip.svelte'
	import { tooltip } from '../lib/tooltip'
	import { store } from '../lib/store.svelte'
	import { activeSource } from '../types'
	import type { LockedMod, Severity } from '../types'
	import { channelOf, sideValue, updateFor } from '../lib/mods'
	import { visibleProblems, hiddenProblems, suppress, restoreAll } from '../lib/problems'

	let { onadd }: { onadd?: () => void } = $props()

	type Filter = 'mod' | 'resourcepack' | 'shader'
	let filter = $state<Filter>('mod')
	let search = $state('')

	const FILTERS: { key: Filter; label: string }[] = [
		{ key: 'mod', label: 'Mods' },
		{ key: 'resourcepack', label: 'Resource packs' },
		{ key: 'shader', label: 'Shaders' },
	]
	const presentTypes = $derived.by(() => {
		const set = new Set<string>()
		for (const m of store.lockfile?.mods ?? []) set.add(m.projectType)
		return set
	})
	const visibleFilters = $derived(FILTERS.filter((f) => presentTypes.has(f.key)))
	$effect(() => {
		if (visibleFilters.length && !presentTypes.has(filter)) filter = visibleFilters[0].key
	})

	let facets = $state({
		providers: [] as string[],
		channels: [] as string[],
		sides: [] as string[],
		kinds: [] as string[],
	})
	const facetCount = $derived(
		facets.providers.length + facets.channels.length + facets.sides.length + facets.kinds.length,
	)
	const presentProviders = $derived.by(() => {
		const set = new Set<string>()
		for (const m of store.lockfile?.mods ?? []) set.add(m.preferred)
		return [...set]
	})

	function matches(mod: LockedMod): boolean {
		if (mod.projectType !== filter) return false
		if (facets.providers.length && !facets.providers.includes(mod.preferred)) return false
		if (facets.channels.length && !facets.channels.includes(channelOf(mod))) return false
		if (facets.sides.length && !facets.sides.includes(sideValue(mod))) return false
		if (facets.kinds.length && !facets.kinds.includes(mod.dependencyType)) return false
		const q = search.trim().toLowerCase()
		if (!q) return true
		const author = store.meta[mod.projectId]?.author?.toLowerCase() ?? ''
		return mod.name.toLowerCase().includes(q) || author.includes(q) || mod.slug.toLowerCase().includes(q)
	}

	const chosen = $derived(store.chosenMods.filter(matches))
	const deps = $derived(store.depMods.filter(matches))
	const selectable = $derived([...chosen, ...deps])
	const isEmpty = $derived((store.lockfile?.mods.length ?? 0) === 0)

	let selected = $state(new Set<string>())
	let selectAnchor = $state<number | null>(null)
	let confirmBulkDelete = $state(false)
	const selectedMods = $derived(selectable.filter((m) => selected.has(m.projectId)))
	const allSelected = $derived(
		selectable.length > 0 && selectable.every((m) => selected.has(m.projectId)),
	)
	const canUpdate = $derived(selectedMods.some((m) => !!updateFor(m)))
	const canPin = $derived(
		selectedMods.some((m) => !activeSource(m)?.pin && !!activeSource(m)?.versionId),
	)
	const canUnpin = $derived(selectedMods.some((m) => !!activeSource(m)?.pin))
	const anyEnabled = $derived(selectedMods.some((m) => !m.disabled))
	const anyDisabled = $derived(selectedMods.some((m) => m.disabled))

	function toggleSelect(id: string) {
		if (selected.has(id)) selected.delete(id)
		else selected.add(id)
		selected = new Set(selected)
		confirmBulkDelete = false
	}
	function clearSelection() {
		selected = new Set()
		selectAnchor = null
		confirmBulkDelete = false
	}
	function selectRow(e: MouseEvent, index: number) {
		const t = e.target as HTMLElement
		if (t.closest('button, a, input, select, textarea, [role="menu"], [role="menuitem"]')) return
		const id = selectable[index].projectId
		if (e.shiftKey && selectAnchor !== null) {
			const lo = Math.min(selectAnchor, index)
			const hi = Math.max(selectAnchor, index)
			const next = new Set<string>()
			for (let i = lo; i <= hi; i++) next.add(selectable[i].projectId)
			selected = next
		} else if (e.metaKey || e.ctrlKey) {
			if (selected.has(id)) selected.delete(id)
			else selected.add(id)
			selected = new Set(selected)
			selectAnchor = index
		} else {
			selected = new Set([id])
			selectAnchor = index
		}
		confirmBulkDelete = false
	}
	function keyRow(e: KeyboardEvent, id: string) {
		if (e.target !== e.currentTarget) return
		if (e.key === 'Enter' || e.key === ' ') {
			e.preventDefault()
			toggleSelect(id)
		}
	}
	function selectAll() {
		if (allSelected) {
			clearSelection()
			return
		}
		selected = new Set(selectable.map((m) => m.projectId))
		selectAnchor = selectable.length ? 0 : null
		confirmBulkDelete = false
	}
	function bulkUpdate() {
		const ups = selectedMods
			.filter((m) => updateFor(m))
			.map((m) => ({ projectId: m.projectId, version: updateFor(m)!.id }))
		if (ups.length) store.setModVersions(ups)
		clearSelection()
	}
	function bulkPin() {
		const ups = selectedMods
			.filter((m) => !activeSource(m)?.pin && activeSource(m)?.versionId)
			.map((m) => ({ projectId: m.projectId, version: activeSource(m)!.versionId }))
		if (ups.length) store.setModVersions(ups)
		clearSelection()
	}
	function bulkUnpin() {
		const ups = selectedMods
			.filter((m) => activeSource(m)?.pin)
			.map((m) => ({ projectId: m.projectId, version: 'latest' }))
		if (ups.length) store.setModVersions(ups)
		clearSelection()
	}
	function bulkDisable() {
		const ids = selectedMods.filter((m) => !m.disabled).map((m) => m.projectId)
		if (ids.length) store.bulkSetDisabled(ids, true)
		clearSelection()
	}
	function bulkEnable() {
		const ids = selectedMods.filter((m) => m.disabled).map((m) => m.projectId)
		if (ids.length) store.bulkSetDisabled(ids, false)
		clearSelection()
	}
	function bulkDelete() {
		const ids = selectedMods.map((m) => m.projectId)
		if (ids.length) store.requestDelete(ids)
		clearSelection()
	}

	const nameMap = $derived.by(() => {
		const m = new Map<string, string>()
		for (const mod of store.lockfile?.mods ?? []) m.set(mod.projectId, mod.name)
		return m
	})
	const nameOf = (id: string) => nameMap.get(id) ?? id

	const updatableMods = $derived(store.chosenMods.filter((m: LockedMod) => updateFor(m)))
	let showUpdates = $state(false)

	let sourceModals = $state<SourceModals>()

	const order: Record<Severity, number> = { error: 0, warning: 1, info: 2, ok: 3 }
	const admonType = { error: 'error', warning: 'warning', info: 'info', ok: 'success' } as const
	const problems = $derived(
		[...visibleProblems()].sort((a, b) => order[a.severity] - order[b.severity]),
	)
	const hidden = $derived(hiddenProblems())
	let showProblems = $state(true)
	let showDeps = $state(true)

	const unpublishedShown = $derived.by(() => {
		if (facetCount > 0) return []
		const q = search.trim().toLowerCase()
		return store.unpublished.filter((u) => {
			if (u.projectType !== filter) return false
			if (q && !u.filename.toLowerCase().includes(q) && !(u.meta?.name?.toLowerCase().includes(q) ?? false))
				return false
			return true
		})
	})
</script>

<div class="h-full flex flex-col min-h-0">
	{#if isEmpty}
		<div class="flex-1 overflow-y-auto flex flex-col">
			<div class="flex-1 flex flex-col items-center justify-center text-center gap-[0.4rem] p-8">
				<div
					class="w-[84px] h-[84px] grid place-items-center rounded-xl bg-bg-inset text-secondary mb-[0.6rem]"
				>
					<Boxes size={40} strokeWidth={1.5} />
				</div>
				<h2 class="text-[1.3rem]">Nothing here yet</h2>
				<p class="text-secondary mb-4 max-w-[340px]">
					Add mods, resource packs, or shaders to get started.
				</p>
				<ButtonStyled color="brand" size="large" disabled={store.busy} onclick={() => onadd?.()}>
					Add content
				</ButtonStyled>
			</div>
		</div>
	{:else}
		<div
			class="flex items-center justify-between gap-4 px-[1.1rem] py-[0.85rem] border-b border-divider"
		>
			<div class="flex gap-[0.35rem] flex-wrap">
				{#each visibleFilters as f (f.key)}
					<button
						class={`border text-[0.78rem] px-3 py-[0.32rem] rounded-max cursor-pointer ${
							filter === f.key
								? 'bg-brand text-on-brand border-brand'
								: 'bg-bg-raised border-divider text-body hover:border-divider-dark'
						}`}
						onclick={() => (filter = f.key)}
					>
						{f.label}
					</button>
				{/each}
			</div>
			<div class="flex items-center gap-[0.6rem]">
				{#if updatableMods.length}
					<button
						class="inline-flex items-center gap-[0.35rem] bg-brand text-on-brand text-[0.76rem] font-semibold px-[0.6rem] py-[0.34rem] rounded-md cursor-pointer hover:bg-brand-hover disabled:opacity-60 disabled:cursor-default"
						disabled={store.busy}
						onclick={() => (showUpdates = true)}
					>
						<RefreshCw size={13} /> Updates<span
							class="bg-white/[0.22] rounded-max px-[0.4rem] py-[0.02rem] text-[0.68rem] ml-[0.1rem]"
							>{updatableMods.length}</span
						>
					</button>
				{/if}
				<ContentFilters {facets} {presentProviders} />
				<div
					class="flex items-center gap-[0.4rem] bg-bg-inset rounded-md px-[0.6rem] py-[0.35rem] text-secondary"
				>
					<Search size={14} />
					<input
						bind:value={search}
						placeholder="Search name or author"
						spellcheck="false"
						class="bg-transparent border-none outline-none text-contrast text-[0.8rem] w-[11rem]"
					/>
				</div>
			</div>
		</div>

		<div class="flex-1 overflow-y-auto px-[1.1rem] pt-[0.6rem] pb-8 max-w-[900px] w-full mx-auto">
			<FeatureTip id="content" class="mb-2">
				View all your downloaded content. Search Modrinth and CurseForge to add mods, resource
				packs, and shaders. Dependencies resolve automatically, and updates show their impact before
				you apply them.
			</FeatureTip>
			{#if selectedMods.length}
				<div
					class="sticky top-0 z-10 flex items-center flex-wrap gap-2 mb-2 bg-bg-super-raised border border-divider rounded-md shadow-raised px-3 py-2"
				>
					<span class="text-[0.82rem] font-semibold text-contrast">{selectedMods.length} selected</span>
					<button
						class="text-[0.78rem] text-secondary bg-transparent border-none cursor-pointer hover:text-contrast hover:underline"
						onclick={selectAll}>{allSelected ? 'Deselect all' : 'Select all'}</button
					>
					<div class="flex-1"></div>
					<ButtonStyled size="small" type="outlined" disabled={store.busy || !canUpdate} onclick={bulkUpdate}>Update</ButtonStyled>
					<ButtonStyled size="small" type="outlined" disabled={store.busy || !canPin} onclick={bulkPin}>Pin</ButtonStyled>
					<ButtonStyled size="small" type="outlined" disabled={store.busy || !canUnpin} onclick={bulkUnpin}>Unpin</ButtonStyled>
					{#if anyDisabled}
						<ButtonStyled size="small" type="outlined" disabled={store.busy} onclick={bulkEnable}>Enable</ButtonStyled>
					{/if}
					{#if anyEnabled}
						<ButtonStyled size="small" type="outlined" disabled={store.busy} onclick={bulkDisable}>Disable</ButtonStyled>
					{/if}
					{#if confirmBulkDelete}
						<span class="inline-flex items-center gap-1">
							<button
								class="bg-red text-white text-[0.72rem] font-semibold px-2 py-[0.3rem] rounded-sm cursor-pointer disabled:opacity-60"
								disabled={store.busy}
								onclick={bulkDelete}>Delete {selectedMods.length}</button
							>
							<button
								class="bg-button-bg text-body text-[0.72rem] px-2 py-[0.3rem] rounded-sm cursor-pointer hover:text-contrast"
								onclick={() => (confirmBulkDelete = false)}>No</button
							>
						</span>
					{:else}
						<ButtonStyled size="small" color="red" type="transparent" disabled={store.busy} onclick={() => (confirmBulkDelete = true)}>Delete</ButtonStyled>
					{/if}
					<button
						use:tooltip={'Clear selection'}
						aria-label="Clear selection"
						class="grid place-items-center w-7 h-7 rounded-sm text-secondary bg-transparent border-none cursor-pointer hover:bg-button-bg hover:text-contrast"
						onclick={clearSelection}><X size={15} /></button
					>
				</div>
			{/if}
			{#if problems.length}
				<div class="mt-2 mb-[0.4rem]">
					<button
						class="flex items-center gap-[0.4rem] bg-transparent border-none text-secondary text-[0.76rem] font-semibold cursor-pointer px-[0.2rem] py-[0.3rem]"
						onclick={() => (showProblems = !showProblems)}
					>
						<ChevronDown
							size={14}
							class={`transition-transform duration-[0.12s] ${showProblems ? '' : '-rotate-90'}`}
						/>
						{problems.length}
						{problems.length === 1 ? 'issue' : 'issues'} to review
					</button>
					{#if showProblems}
						<div class="flex flex-col gap-2 mt-[0.4rem] mb-[0.8rem]">
							{#each problems as v (v.id)}
								<div class="relative group">
									<Admonition type={admonType[v.severity]} title={v.title}>
										{v.detail}
										{#if v.fix && v.fix.kind !== 'none'}
											<div class="mt-2">
												<ButtonStyled
													size="small"
													type="outlined"
													disabled={store.busy}
													onclick={() => store.applyFix(v.fix!)}
												>
													{v.fix.label}
												</ButtonStyled>
											</div>
										{/if}
									</Admonition>
									<button
										class="absolute top-[0.55rem] right-[0.55rem] grid place-items-center w-6 h-6 rounded-sm text-secondary bg-transparent border-none cursor-pointer opacity-0 group-hover:opacity-100 hover:bg-button-bg hover:text-contrast"
										use:tooltip={'Hide this problem'}
										aria-label="Hide this problem"
										onclick={() => suppress(v)}
									>
										<EyeOff size={14} />
									</button>
								</div>
							{/each}
						</div>
					{/if}
				</div>
			{/if}
			{#if hidden.length}
				<button
					class="bg-transparent border-none text-secondary text-[0.74rem] cursor-pointer px-[0.2rem] py-[0.2rem] mb-[0.4rem] hover:text-body"
					onclick={() => restoreAll()}
				>
					Show {hidden.length} hidden {hidden.length === 1 ? 'problem' : 'problems'}
				</button>
			{/if}

			{#if chosen.length}
				<div
					class="text-[0.68rem] uppercase tracking-[0.06em] text-secondary font-[650] px-[0.3rem] pt-[0.9rem] pb-[0.45rem] flex items-center gap-2"
				>
					In your pack
				</div>
			{/if}
			<ul class="list-none m-0 p-0">
				{#each chosen as mod, modIndex (mod.projectId)}
					<ModRow
						{mod}
						index={modIndex}
						selected={selected.has(mod.projectId)}
						onselect={selectRow}
						onkeyrow={keyRow}
						onaddalt={(m, t) => sourceModals?.start(m, t)}
					/>
				{/each}
			</ul>

			{#if deps.length}
				<button
					class="w-full text-left bg-transparent border-none cursor-pointer text-[0.68rem] uppercase tracking-[0.06em] text-secondary font-[650] px-[0.3rem] pt-[0.9rem] pb-[0.45rem] flex items-center gap-2 hover:text-body"
					onclick={() => (showDeps = !showDeps)}
				>
					Pulled in as dependencies <span
						class="bg-button-bg text-secondary rounded-max px-[0.45rem] py-[0.05rem] text-[0.62rem]"
						>{deps.length}</span
					>
					<ChevronDown
						size={13}
						class={`transition-transform duration-[0.12s] ${showDeps ? '' : '-rotate-90'}`}
					/>
				</button>
				{#if showDeps}
					<ul class="list-none m-0 p-0">
						{#each deps as mod, depIndex (mod.projectId)}
							<DepRow
								{mod}
								index={chosen.length + depIndex}
								selected={selected.has(mod.projectId)}
								{nameOf}
								onselect={selectRow}
								onkeyrow={keyRow}
							/>
						{/each}
					</ul>
				{/if}
			{/if}

			{#if !chosen.length && !deps.length && !unpublishedShown.length}
				<div class="text-center text-secondary px-4 py-10 text-[0.85rem]">
					No content matches this filter.
				</div>
			{/if}

			{#if unpublishedShown.length}
				<UnpublishedSection items={unpublishedShown} />
			{/if}
		</div>
	{/if}
</div>

<SourceModals bind:this={sourceModals} />

{#if showUpdates}
	<UpdatesModal onclose={() => (showUpdates = false)} />
{/if}

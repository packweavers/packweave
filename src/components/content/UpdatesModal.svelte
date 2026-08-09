<script lang="ts">
	import { ArrowRight, Plus, TriangleAlert } from '@lucide/svelte'
	import Avatar from '../ui/Avatar.svelte'
	import ButtonStyled from '../ui/ButtonStyled.svelte'
	import Modal from '../ui/Modal.svelte'
	import { api } from '../../api'
	import { store } from '../../lib/store.svelte'
	import { activeSource } from '../../types'
	import type { ImpactReport, LockedMod, VersionInfo } from '../../types'
	import { prerelease, updateFor } from '../../lib/mods'

	let { onclose }: { onclose: () => void } = $props()

	let updateList = $state<{ mod: LockedMod; latest: VersionInfo; checked: boolean }[]>(
		store.chosenMods
			.filter((m) => updateFor(m))
			.map((m) => ({ mod: m, latest: updateFor(m)!, checked: true })),
	)
	const checkedUpdates = $derived(updateList.filter((u) => u.checked))
	let impact = $state<ImpactReport | null>(null)
	let impactLoading = $state(false)
	const addedDeps = $derived(impact?.changes.filter((c) => c.kind === 'added') ?? [])

	async function loadImpact() {
		if (!store.pack || !checkedUpdates.length) {
			impact = null
			return
		}
		const updates = checkedUpdates.map((u) => ({ projectId: u.mod.projectId, version: u.latest.id }))
		impactLoading = true
		try {
			impact = await api.updateImpact(store.pack.dir, updates)
		} catch {
			impact = null
		} finally {
			impactLoading = false
		}
	}

	$effect(() => {
		checkedUpdates
		loadImpact()
	})

	async function confirmUpdates() {
		const updates = checkedUpdates.map((u) => ({ projectId: u.mod.projectId, version: u.latest.id }))
		onclose()
		await store.setModVersions(updates)
	}
</script>

<Modal title="Update content" {onclose}>
	<div class="flex flex-col gap-1 max-h-[340px] overflow-y-auto mb-4">
		{#each updateList as u (u.mod.projectId)}
			<label
				class="flex items-center gap-2.5 px-md py-md rounded-md cursor-pointer hover:bg-bg-raised"
			>
				<input
					type="checkbox"
					bind:checked={u.checked}
					class="w-4 h-4 [accent-color:var(--color-brand)] cursor-pointer shrink-0"
				/>
				<Avatar src={store.meta[u.mod.projectId]?.iconUrl ?? null} alt={u.mod.name} size={32} />
				<div class="flex-1 min-w-0">
					<div class="font-medium text-contrast whitespace-nowrap overflow-hidden text-ellipsis">
						{u.mod.name}
					</div>
					<div
						class="flex items-center flex-wrap gap-sm mt-2xs text-tiny text-secondary font-mono"
					>
						<span class="[overflow-wrap:anywhere] min-w-0">{activeSource(u.mod)?.versionNumber}</span>
						<ArrowRight size={12} />
						<span class="[overflow-wrap:anywhere] min-w-0 text-green">{u.latest.versionNumber}</span>
						{#if prerelease(u.latest)}
							<span
								class={`shrink-0 text-micro font-bold uppercase tracking-[0.03em] px-md py-2xs rounded-max ${
									u.latest.versionType === 'beta' ? 'text-orange bg-orange/15' : 'text-red bg-red/15'
								}`}
							>
								{u.latest.versionType}
							</span>
						{/if}
					</div>
				</div>
			</label>
		{/each}
	</div>
	{#if checkedUpdates.some((u) => prerelease(u.latest))}
		<div class="flex items-start gap-md mt-3 text-small text-orange">
			<TriangleAlert size={14} />
			Some of these are beta or alpha pre-release builds.
		</div>
	{/if}

	{#if impactLoading}
		<div class="mt-3 text-secondary text-small">Checking impact…</div>
	{:else if impact}
		{#if addedDeps.length}
			<div class="mt-3">
				<div class="text-tiny uppercase tracking-[0.04em] font-[650] text-secondary mb-sm">
					Also pulls in {addedDeps.length} new {addedDeps.length === 1 ? 'dependency' : 'dependencies'}
				</div>
				{#each addedDeps as c (c.name)}
					<div class="flex items-center gap-sm text-small text-body py-2xs">
						<Plus size={12} /> {c.name}{#if c.toVersion}<span
								class="ml-sm text-tiny text-secondary font-mono">{c.toVersion}</span
							>{/if}
					</div>
				{/each}
			</div>
		{/if}
		{#if impact.problems.length}
			<div class="flex items-start gap-md mt-3 text-small text-orange">
				<TriangleAlert size={14} />
				<div class="flex flex-col gap-2xs">
					{#each impact.problems as p, i (i)}
						<div>{p}</div>
					{/each}
				</div>
			</div>
		{/if}
	{/if}

	<div class="flex justify-end items-center gap-2.5 mt-4">
		<ButtonStyled type="transparent" disabled={store.busy} onclick={onclose}>Cancel</ButtonStyled>
		<ButtonStyled color="brand" disabled={!checkedUpdates.length || store.busy} onclick={confirmUpdates}>
			Update {checkedUpdates.length}
		</ButtonStyled>
	</div>
</Modal>

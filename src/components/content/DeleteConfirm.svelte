<script lang="ts">
	import Modal from '../ui/Modal.svelte'
	import ButtonStyled from '../ui/ButtonStyled.svelte'
	import { store } from '../../lib/store.svelte'

	const ids = $derived(store.deletePrompt?.ids ?? [])
	const mods = $derived.by(() => {
		const byId = new Map((store.lockfile?.mods ?? []).map((m) => [m.projectId, m]))
		return ids.map((id) => byId.get(id)).filter((m) => !!m)
	})

	function names(depIds: string[]): string {
		const byId = new Map((store.lockfile?.mods ?? []).map((m) => [m.projectId, m.name]))
		const list = depIds.map((d) => byId.get(d) ?? d)
		if (list.length <= 1) return list[0] ?? 'another mod'
		if (list.length === 2) return `${list[0]} and ${list[1]}`
		return `${list.slice(0, -1).join(', ')}, and ${list[list.length - 1]}`
	}

	const idSet = $derived(new Set(ids))
	const neededByOf = (m: (typeof mods)[number]) =>
		m.dependents.filter((d) => !idSet.has(d))

	const single = $derived(mods.length === 1 ? mods[0] : null)
	const singleExplicit = $derived(single?.dependencyType === 'explicit')
	const neededCount = $derived(mods.filter((m) => neededByOf(m).length > 0).length)

	const title = $derived(
		single ? `Delete ${single.name}?` : `Delete ${ids.length} items?`,
	)
</script>

<Modal {title} onclose={() => store.dismissDeletePrompt()}>
	<p class="text-[0.85rem] text-body leading-[1.5] mb-[1.2rem]">
		{#if single}
			{names(neededByOf(single))} needs it.
			{#if singleExplicit}
				Delete it anyway, or keep it as a dependency.
			{:else}
				Delete it anyway?
			{/if}
		{:else}
			{neededCount}
			{neededCount === 1 ? 'of these is a dependency' : 'of these are dependencies'} other mods still
			need.
		{/if}
	</p>
	<div class="flex justify-end items-center gap-[0.6rem]">
		<ButtonStyled type="transparent" disabled={store.busy} onclick={() => store.dismissDeletePrompt()}>
			Cancel
		</ButtonStyled>
		{#if single && singleExplicit}
			<ButtonStyled
				type="outlined"
				disabled={store.busy}
				onclick={() => store.confirmDeletePrompt('dependency')}
			>
				Set as dependency
			</ButtonStyled>
		{/if}
		<ButtonStyled color="red" disabled={store.busy} onclick={() => store.confirmDeletePrompt('delete')}>
			Delete
		</ButtonStyled>
	</div>
</Modal>

<script lang="ts">
	import { content } from '@/lib/state/content.svelte'
	import type { ImpactReport } from '@/lib/tauri/types'
	import Dialog from '@/ui/Dialog.svelte'

	interface Props {
		keys: string[]
		impact: ImpactReport | null
		close: (result?: 'remove' | 'disable') => void
	}

	let { keys, impact, close }: Props = $props()

	const names = $derived(keys.map((k) => content.find(k)?.name ?? k))
	const removed = $derived((impact?.changes ?? []).filter((c) => c.kind === 'removed'))
</script>

<Dialog
	title="Remove {names.length === 1 ? names[0] : `${names.length} items`}?"
	onclose={() => close()}
>
	<div class="list">
		{#each removed as change (change.key)}
			<div class="trow">
				<span class="mark removed"></span>
				<span class="name">{change.name}</span>
				{#if change.dependency}<span class="tag">dependency</span>{/if}
				<span class="spacer"></span>
				<span class="mono">{change.fromVersion ?? ''}</span>
			</div>
		{/each}
	</div>
	{#snippet footer()}
		<button class="btn" onclick={() => close('disable')}>Disable</button>
		<span class="spacer"></span>
		<button class="btn" onclick={() => close()}>Cancel</button>
		<button class="btn danger-fill" onclick={() => close('remove')}
			>Remove {removed.length || keys.length}</button
		>
	{/snippet}
</Dialog>

<style>
	.list {
		border: 1px solid var(--line);
		border-bottom: 0;
	}
</style>

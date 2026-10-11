<script lang="ts">
	import { untrack } from 'svelte'

	import type { DeletedBranchInstance } from '@/lib/state/prefs'
	import { plural } from '@/lib/util/format'
	import { instanceFolderName } from '@/lib/util/path'
	import Dialog from '@/ui/Dialog.svelte'
	import Segmented from '@/ui/Segmented.svelte'

	type InstanceChoice = 'delete' | 'link' | 'keep'

	interface Props {
		from: string
		into: string
		commits: number
		conflicts: string[] | null
		hasFolder: boolean
		instanceDir: string | null
		policy: DeletedBranchInstance
		hasRemote: boolean
		remote: boolean
		close: (result?: {
			mode: 'merge' | 'squash'
			push: boolean
			deleteBranch: boolean
			instance: InstanceChoice
		}) => void
	}

	let {
		from,
		into,
		commits,
		conflicts,
		hasFolder,
		instanceDir,
		hasRemote,
		remote,
		policy,
		close,
	}: Props = $props()

	let mode = $state<'merge' | 'squash'>('merge')
	let push = $state(untrack(() => hasRemote))
	let deleteBranch = $state(true)
	let instance = $state<InstanceChoice>(untrack(() => (policy === 'delete' ? 'delete' : 'keep')))

	const instanceName = $derived(instanceDir ? instanceFolderName(instanceDir) : null)
</script>

<Dialog title="Merge {from} into {into}" width={560} onclose={() => close()}>
	<p>
		{plural(commits, 'commit', 'commits')} ·
		{#if conflicts === null}
			conflicts unknown
		{:else if conflicts.length === 0}
			<span class="added">✓</span> no conflicts
		{:else}
			<span class="removed">✗</span> {plural(conflicts.length, 'conflict', 'conflicts')} to settle
		{/if}
	</p>
	<div class="band">How</div>
	<Segmented
		options={[
			{ value: 'merge', label: 'Merge commit' },
			{ value: 'squash', label: 'Squash' },
		]}
		bind:value={mode}
	/>
	<div class="band">Then</div>
	{#if hasRemote}
		<label class="check"><input type="checkbox" bind:checked={push} />Push {into} to origin</label>
	{/if}
	{#if !remote}
		<label class="check">
			<input type="checkbox" bind:checked={deleteBranch} />Delete {from}{hasFolder
				? ' and its folder'
				: ''}
		</label>
	{/if}
	{#if instanceName}
		<div class="band">{instanceName}</div>
		<Segmented
			options={[
				{ value: 'delete', label: 'Delete' },
				{ value: 'link', label: `Link to ${into}` },
				{ value: 'keep', label: 'Keep, unlinked' },
			]}
			bind:value={instance}
		/>
	{/if}
	{#snippet footer()}
		<button class="btn" onclick={() => close()}>Cancel</button>
		<button
			class="btn primary"
			onclick={() => close({ mode, push, deleteBranch: deleteBranch && !remote, instance })}
			>Merge</button
		>
	{/snippet}
</Dialog>

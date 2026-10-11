<script lang="ts">
	import { untrack } from 'svelte'

	import type { DeletedBranchInstance } from '@/lib/state/prefs'
	import { plural } from '@/lib/util/format'
	import { instanceFolderName } from '@/lib/util/path'
	import Dialog from '@/ui/Dialog.svelte'

	interface Props {
		branch: string
		base: string
		unmerged: number
		hasFolder: boolean
		instanceDir: string | null
		canDeleteInstance: boolean
		policy: DeletedBranchInstance
		close: (result?: { deleteInstance: boolean }) => void
	}

	let { branch, base, unmerged, hasFolder, instanceDir, canDeleteInstance, policy, close }: Props =
		$props()

	let deleteInstance = $state(untrack(() => policy === 'delete'))
</script>

<Dialog title="Delete branch {branch}?" onclose={() => close()}>
	<p>
		{#if unmerged > 0}{plural(unmerged, 'commit', 'commits')} not on {base}.{:else}Merged into {base}.{/if}
		{#if hasFolder}Deletes its folder too.{/if}
	</p>
	{#if instanceDir && canDeleteInstance}
		{#if policy === 'ask'}
			<label class="check">
				<input type="checkbox" bind:checked={deleteInstance} />
				Also delete {instanceFolderName(instanceDir)}
			</label>
		{:else if policy === 'delete'}
			<p>Also deletes {instanceFolderName(instanceDir)}.</p>
		{:else}
			<p>Keeps {instanceFolderName(instanceDir)}.</p>
		{/if}
	{/if}
	{#snippet footer()}
		<button class="btn" onclick={() => close()}>Cancel</button>
		<button class="btn danger-fill" onclick={() => close({ deleteInstance })}>Delete branch</button>
	{/snippet}
</Dialog>

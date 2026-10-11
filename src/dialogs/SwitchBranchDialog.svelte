<script lang="ts">
	import { git } from '@/lib/state/git.svelte'
	import { plural } from '@/lib/util/format'
	import Dialog from '@/ui/Dialog.svelte'

	interface Props {
		branch: string
		changes: number
		close: (result?: 'commit' | 'stash' | 'bring') => void
	}

	let { branch, changes, close }: Props = $props()
</script>

<Dialog title="Switch to {branch}?" onclose={() => close()}>
	<p>{git.branch} has {plural(changes, 'uncommitted change', 'uncommitted changes')}.</p>
	{#snippet footer()}
		<button class="btn" onclick={() => close('commit')}>Commit</button>
		<span class="spacer"></span>
		<button class="btn" onclick={() => close()}>Cancel</button>
		<button class="btn" onclick={() => close('stash')}>Stash</button>
		<button class="btn primary" onclick={() => close('bring')}>Bring them along</button>
	{/snippet}
</Dialog>

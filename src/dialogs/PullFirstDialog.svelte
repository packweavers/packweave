<script lang="ts">
	import { plural } from '@/lib/util/format'
	import Dialog from '@/ui/Dialog.svelte'

	interface Props {
		upstream: string
		behind: number
		close: (result?: 'pull' | 'commit') => void
	}

	let { upstream, behind, close }: Props = $props()
</script>

<Dialog title="Pull first?" onclose={() => close()}>
	<p>{upstream} has {plural(behind, 'commit', 'commits')} you haven't pulled.</p>
	{#snippet footer()}
		<button class="btn" onclick={() => close()}>Cancel</button>
		<button class="btn" onclick={() => close('commit')}>Commit anyway</button>
		<button class="btn primary" onclick={() => close('pull')}>Pull</button>
	{/snippet}
</Dialog>

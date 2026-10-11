<script lang="ts">
	import { dialogs } from '@/lib/state/dialogs.svelte'

	const open = $derived(dialogs.current)
</script>

{#if open}
	{@const Current = open.component}
	<Current {...open.props} close={(result?: unknown) => dialogs.close(result)} />
{/if}

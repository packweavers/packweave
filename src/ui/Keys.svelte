<script lang="ts">
	import { keysFor } from '@/lib/state/commands.svelte'

	interface Props {
		command?: string
		keys?: string
	}

	let { command, keys }: Props = $props()

	const label = $derived(keys ?? (command ? keysFor(command) : null))
</script>

{#if label}
	<span class="kbd">{label}</span>
{/if}

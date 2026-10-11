<script lang="ts">
	import type { Doc } from '@/lib/state/tabs.svelte'
	import GameLog from '@/panel/GameLog.svelte'
	import Output from '@/panel/Output.svelte'
	import Problems from '@/panel/Problems.svelte'

	interface Props {
		doc: Doc
	}

	let { doc }: Props = $props()
</script>

{#if doc.kind === 'problems'}
	<Problems />
{:else if doc.kind === 'output'}
	<Output />
{:else}
	<GameLog />
{/if}

<script lang="ts">
	import { tasks } from '@/lib/state/tasks.svelte'

	let box = $state<HTMLDivElement | null>(null)

	$effect(() => {
		void tasks.log.length
		if (box) box.scrollTop = box.scrollHeight
	})

	function time(ms: number): string {
		return new Date(ms).toLocaleTimeString([], {
			hour: '2-digit',
			minute: '2-digit',
			second: '2-digit',
		})
	}
</script>

<div class="bar sub-toolbar">
	<span class="small">{tasks.running.length ? `${tasks.running.length} running` : 'Idle'}</span>
	<span class="spacer"></span>
	<button class="btn" disabled={tasks.log.length === 0} onclick={() => tasks.clear()}>Clear</button>
</div>
<div class="scroll log" bind:this={box}>
	{#each tasks.log as entry (entry.id)}
		<div class="line code" class:error={entry.tone === 'error'}>
			<span class="time">{time(entry.time)}</span>
			<span>{entry.text}</span>
		</div>
	{/each}
</div>

<style>
	.log {
		padding: 4px 0;
		user-select: text;
		-webkit-user-select: text;
	}

	.line {
		display: flex;
		gap: 12px;
		padding: 0 14px;
		white-space: pre-wrap;
	}

	.line.error {
		color: var(--red);
	}

	.time {
		flex: none;
	}
</style>

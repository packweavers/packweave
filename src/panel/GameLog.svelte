<script lang="ts">
	import { onMount } from 'svelte'

	import { instance } from '@/lib/state/instance.svelte'
	import { toasts } from '@/lib/state/toasts.svelte'
	import { openFolder } from '@/lib/tauri/windows'
	import { join } from '@/lib/util/path'

	let box = $state<HTMLPreElement | null>(null)
	let follow = $state(true)

	onMount(() => {
		instance.followLog(true)
		return () => instance.followLog(false)
	})

	$effect(() => {
		void instance.log
		if (follow && box) box.scrollTop = box.scrollHeight
	})

	function onscroll(): void {
		if (!box) return
		follow = box.scrollTop + box.clientHeight >= box.scrollHeight - 40
	}

	function copy(): void {
		navigator.clipboard
			.writeText(instance.log)
			.then(() => toasts.show('Copied the log'))
			.catch((error) => toasts.error(error))
	}
</script>

<div class="bar sub-toolbar">
	<span class="small">{instance.bound ? `${instance.name} · logs/latest.log` : 'No instance'}</span>
	<span class="spacer"></span>
	<button class="btn" disabled={!instance.log} onclick={copy}>Copy</button>
	<button
		class="btn"
		disabled={!instance.binding}
		onclick={() => instance.binding && void openFolder(join(instance.binding.gameDir, 'logs'))}
		>Open folder</button
	>
</div>
{#if instance.logError}
	<div class="banner error">Couldn't read the log: {instance.logError}</div>
{/if}
<pre class="scroll log code" bind:this={box} {onscroll}>{instance.log ||
		(instance.bound ? 'Nothing logged yet' : '')}</pre>

<style>
	.log {
		margin: 0;
		padding: 6px 14px;
		user-select: text;
		-webkit-user-select: text;
		white-space: pre-wrap;
		overflow-wrap: anywhere;
	}
</style>

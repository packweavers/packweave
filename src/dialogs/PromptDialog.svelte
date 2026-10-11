<script lang="ts">
	import { untrack } from 'svelte'

	import Dialog from '@/ui/Dialog.svelte'
	import PathField from '@/ui/PathField.svelte'
	import TextField from '@/ui/TextField.svelte'

	interface Props {
		title: string
		label: string
		value?: string
		placeholder?: string
		fallback?: string
		confirm: string
		suggest?: (text: string) => Promise<string[]>
		close: (result?: string) => void
	}

	let {
		title,
		label,
		value = '',
		placeholder = '',
		fallback = '',
		confirm,
		suggest,
		close,
	}: Props = $props()

	let text = $state(untrack(() => value))

	function submit(): void {
		if (text.trim() || fallback) close(text.trim() || fallback)
	}
</script>

<Dialog {title} onclose={() => close()}>
	<div class="form-field">
		<span class="label">{label}</span>
		{#if suggest}
			<PathField
				bind:value={text}
				placeholder={fallback || placeholder}
				prefilled={!!fallback}
				autofocus
				{suggest}
				onsubmit={submit}
			/>
		{:else}
			<TextField
				bind:value={text}
				placeholder={fallback || placeholder}
				prefilled={!!fallback}
				autofocus
				onkeydown={(e) => e.key === 'Enter' && submit()}
			/>
		{/if}
	</div>
	{#snippet footer()}
		<button class="btn" onclick={() => close()}>Cancel</button>
		<button class="btn primary" onclick={submit}>{confirm}</button>
	{/snippet}
</Dialog>

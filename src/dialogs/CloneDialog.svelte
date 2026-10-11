<script lang="ts">
	import Dialog from '@/ui/Dialog.svelte'
	import Segmented from '@/ui/Segmented.svelte'
	import TextField from '@/ui/TextField.svelte'

	import FolderField from './FolderField.svelte'

	interface Result {
		url: string
		parent: string
		then: 'link' | 'create' | 'skip'
	}

	interface Props {
		close: (result?: Result) => void
	}

	let { close }: Props = $props()

	let url = $state('')
	let parent = $state('')
	let then = $state<'link' | 'create' | 'skip'>('link')

	function submit(): void {
		if (url.trim() && parent) close({ url: url.trim(), parent, then })
	}
</script>

<Dialog title="Clone a pack" onclose={() => close()}>
	<div class="form-field">
		<span class="label">Repository</span>
		<TextField
			bind:value={url}
			placeholder="https://github.com/team/pack"
			mono
			autofocus
			onkeydown={(e) => e.key === 'Enter' && submit()}
		/>
	</div>
	<div class="form-field">
		<span class="label">Clone into</span>
		<FolderField bind:value={parent} title="Choose where to clone the pack" />
	</div>
	<div class="form-field">
		<span class="label">Then</span>
		<Segmented
			options={[
				{ value: 'link', label: 'Link an instance' },
				{ value: 'create', label: 'Create one' },
				{ value: 'skip', label: 'Skip' },
			]}
			bind:value={then}
		/>
	</div>
	{#snippet footer()}
		<button class="btn" onclick={() => close()}>Cancel</button>
		<button class="btn primary" disabled={!url.trim() || !parent} onclick={submit}>Clone</button>
	{/snippet}
</Dialog>

<script lang="ts">
	import { untrack } from 'svelte'

	import { pickFile } from '@/lib/tauri/windows'
	import { basename, dirname, join } from '@/lib/util/path'
	import { slugify } from '@/lib/util/text'
	import Dialog from '@/ui/Dialog.svelte'

	import FolderField from './FolderField.svelte'

	interface Result {
		source: string
		dest: string
		git: boolean
	}

	interface Props {
		source: string | null
		close: (result?: Result) => void
	}

	let { source: initial, close }: Props = $props()

	let source = $state(untrack(() => initial ?? ''))
	let parent = $state('')
	let useGit = $state(true)

	const packwiz = $derived(basename(source).toLowerCase() === 'pack.toml')
	const stem = $derived(
		packwiz ? basename(dirname(source)) : basename(source).replace(/\.(mrpack|zip)$/i, ''),
	)

	async function choose(): Promise<void> {
		const picked = await pickFile('Choose a pack to import', [
			{ name: 'Modpacks', extensions: ['mrpack', 'zip', 'toml'] },
		])
		if (picked) source = picked
	}

	function submit(): void {
		if (source && parent) close({ source, dest: join(parent, slugify(stem)), git: useGit })
	}
</script>

<Dialog title="Import a pack" onclose={() => close()}>
	<button class="drop" onclick={choose}>
		{#if source}
			<span class="medium">{packwiz ? `${stem} (packwiz)` : basename(source)}</span>
			<span class="small">Choose another</span>
		{:else}
			<span>.mrpack, CurseForge .zip, Prism / MultiMC .zip or a packwiz pack.toml</span>
			<span class="small">Choose a file</span>
		{/if}
	</button>
	<div class="form-field">
		<span class="label">Import into</span>
		<FolderField bind:value={parent} title="Choose where the pack folder goes" />
	</div>
	<label class="check"><input type="checkbox" bind:checked={useGit} />Use Git</label>
	{#snippet footer()}
		<button class="btn" onclick={() => close()}>Cancel</button>
		<button class="btn primary" disabled={!source || !parent} onclick={submit}>Import</button>
	{/snippet}
</Dialog>

<style>
	.drop {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 4px;
		padding: 22px;
		border: 1px dashed var(--edge);
		background: var(--band);
	}

	.drop:hover {
		background: var(--fill);
	}
</style>

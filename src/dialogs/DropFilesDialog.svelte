<script lang="ts">
	import { untrack } from 'svelte'

	import type { DroppedFile } from '@/lib/tauri/types'
	import { providerName, typeName } from '@/lib/util/format'
	import Dialog from '@/ui/Dialog.svelte'

	interface Props {
		files: DroppedFile[]
		close: (result?: DroppedFile[]) => void
	}

	let { files, close }: Props = $props()

	let picked = $state<Record<string, boolean>>(
		untrack(() => Object.fromEntries(files.map((f) => [f.path, !f.alreadyInPack]))),
	)

	const chosen = $derived(files.filter((f) => picked[f.path]))
</script>

<Dialog
	title="Add {files.length === 1 ? files[0].filename : `${files.length} dropped files`}"
	width={560}
	onclose={() => close()}
>
	<div class="list">
		{#each files as file (file.path)}
			<label class="trow tall">
				<input type="checkbox" bind:checked={picked[file.path]} disabled={file.alreadyInPack} />
				<span class="stack">
					<span class="mono ellipsis">{file.filename}</span>
					<span class="small">
						{#if file.matched}
							{file.matched.name} · {providerName(file.matched.provider)}
						{:else}
							{typeName(file.projectType).toLowerCase()} · kept as a file
						{/if}
					</span>
				</span>
				<span class="spacer"></span>
				{#if file.alreadyInPack}
					<span class="tag">in pack</span>
				{:else if !file.matched}
					<span class="tag changed">not found</span>
				{/if}
			</label>
		{/each}
	</div>
	{#snippet footer()}
		<button class="btn" onclick={() => close()}>Cancel</button>
		<button class="btn primary" disabled={chosen.length === 0} onclick={() => close(chosen)}
			>Add {chosen.length}</button
		>
	{/snippet}
</Dialog>

<style>
	.list {
		max-height: 320px;
		overflow: auto;
		border: 1px solid var(--line);
	}

	.tall {
		height: 44px;
	}

	.stack {
		display: flex;
		flex-direction: column;
		min-width: 0;
	}
</style>

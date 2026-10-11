<script lang="ts">
	import { onMount } from 'svelte'

	import { toasts } from '@/lib/state/toasts.svelte'
	import { detectInstances, resolveInstanceFolder } from '@/lib/tauri/instance'
	import type { DetectedInstance } from '@/lib/tauri/types'
	import { pickFolder } from '@/lib/tauri/windows'
	import { loaderName } from '@/lib/util/format'
	import Dialog from '@/ui/Dialog.svelte'

	import FolderField from './FolderField.svelte'

	interface Result {
		instance: DetectedInstance
		parent: string
		git: boolean
	}

	interface Props {
		close: (result?: Result) => void
	}

	let { close }: Props = $props()

	let detected = $state<DetectedInstance[]>([])
	let picked = $state<DetectedInstance | null>(null)
	let parent = $state('')
	let useGit = $state(true)
	let loading = $state(true)

	onMount(() => {
		void detectInstances()
			.then((found) => (detected = found))
			.catch((error) => toasts.error(error))
			.finally(() => (loading = false))
	})

	const launchers = $derived([...new Set(detected.map((d) => d.launcher))])

	async function browse(): Promise<void> {
		const folder = await pickFolder('Choose an instance folder')
		if (!folder) return
		const found = await resolveInstanceFolder(folder)
		detected = [found, ...detected.filter((d) => d.gameDir !== found.gameDir)]
		picked = found
	}

	function submit(): void {
		if (picked && parent) close({ instance: picked, parent, git: useGit })
	}
</script>

<Dialog title="Pack from an instance" width={560} onclose={() => close()}>
	<div class="list">
		{#if loading}
			<div class="trow"><span class="spinner"></span></div>
		{:else if detected.length === 0}
			<div class="trow">No instances found</div>
		{/if}
		{#each launchers as launcher (launcher)}
			<div class="band">{launcher}</div>
			{#each detected.filter((d) => d.launcher === launcher) as found (found.gameDir)}
				<button
					class="trow"
					class:selected={picked?.gameDir === found.gameDir}
					onclick={() => (picked = found)}
				>
					<span class="name">{found.name}</span>
					<span class="spacer"></span>
					<span class="small"
						>{found.minecraft ?? '?'} · {loaderName(found.loader ?? 'vanilla')}{found.loaderVersion
							? ` ${found.loaderVersion}`
							: ''}</span
					>
				</button>
			{/each}
		{/each}
	</div>
	<button class="btn small folder-btn" onclick={browse}>Folder…</button>
	<div class="form-field">
		<span class="label">Pack folder</span>
		<FolderField bind:value={parent} title="Choose where the pack folder goes" />
	</div>
	<label class="check"><input type="checkbox" bind:checked={useGit} />Use Git</label>
	{#snippet footer()}
		<button class="btn" onclick={() => close()}>Cancel</button>
		<button class="btn primary" disabled={!picked || !parent} onclick={submit}>Make a pack</button>
	{/snippet}
</Dialog>

<style>
	.list {
		max-height: 300px;
		overflow: auto;
		border: 1px solid var(--line);
	}

	.list .trow {
		width: 100%;
		text-align: left;
	}

	.folder-btn {
		align-self: flex-start;
	}
</style>

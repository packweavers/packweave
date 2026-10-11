<script lang="ts">
	import { onMount, untrack } from 'svelte'

	import { toasts } from '@/lib/state/toasts.svelte'
	import { detectInstances, resolveInstanceFolder } from '@/lib/tauri/instance'
	import type { DetectedInstance, Manifest } from '@/lib/tauri/types'
	import { pickFolder } from '@/lib/tauri/windows'
	import { loaderName } from '@/lib/util/format'
	import { instanceFolderName } from '@/lib/util/path'
	import Dialog from '@/ui/Dialog.svelte'
	import TextField from '@/ui/TextField.svelte'

	type Result =
		| { action: 'copy'; source: string; name: string }
		| { action: 'move'; source: string }
		| { action: 'use'; gameDir: string }
		| { action: 'create' }

	interface Props {
		target: string
		manifest: Manifest
		mainInstance: string | null
		canCopy: boolean
		close: (result?: Result) => void
	}

	let { target, manifest, mainInstance, canCopy, close }: Props = $props()

	const mainName = $derived(mainInstance ? instanceFolderName(mainInstance) : '')
	let choice = $state<'copy' | 'move' | 'use' | 'create'>(
		untrack(() => (mainInstance && canCopy ? 'copy' : 'use')),
	)
	let copyName = $state(untrack(() => `${manifest.name} ${target}`))
	let detected = $state<DetectedInstance[]>([])
	let picked = $state<string | null>(null)
	let loading = $state(true)

	onMount(() => {
		void detectInstances()
			.then((found) => (detected = found.filter((d) => d.gameDir !== mainInstance)))
			.catch((error) => toasts.error(error))
			.finally(() => (loading = false))
	})

	const launchers = $derived([...new Set(detected.map((d) => d.launcher))])

	function compatible(found: DetectedInstance): boolean {
		return found.minecraft === manifest.minecraft && (found.loader ?? 'vanilla') === manifest.loader
	}

	async function browse(): Promise<void> {
		const folder = await pickFolder('Choose an instance folder')
		if (!folder) return
		const found = await resolveInstanceFolder(folder)
		detected = [found, ...detected.filter((d) => d.gameDir !== found.gameDir)]
		picked = found.gameDir
		choice = 'use'
	}

	function submit(): void {
		if (choice === 'create') {
			close({ action: 'create' })
		} else if (choice === 'use') {
			if (picked) close({ action: 'use', gameDir: picked })
		} else if (mainInstance) {
			close(
				choice === 'copy'
					? { action: 'copy', source: mainInstance, name: copyName.trim() || target }
					: { action: 'move', source: mainInstance },
			)
		}
	}
</script>

<Dialog title="Link an instance to {target}" width={560} onclose={() => close()}>
	{#if mainInstance}
		{#if canCopy}
			<label class="option" class:on={choice === 'copy'}>
				<input type="radio" bind:group={choice} value="copy" />
				<span class="medium">Copy {mainName}</span>
				{#if choice === 'copy'}
					<TextField bind:value={copyName} class="grow" />
				{/if}
			</label>
		{/if}
		<label class="option" class:on={choice === 'move'}>
			<input type="radio" bind:group={choice} value="move" />
			<span class="medium">Move {mainName} here</span>
		</label>
	{/if}
	<label class="option" class:on={choice === 'create'}>
		<input type="radio" bind:group={choice} value="create" />
		<span class="medium">Create a Prism instance</span>
	</label>
	<label class="option" class:on={choice === 'use'}>
		<input type="radio" bind:group={choice} value="use" />
		<span class="medium">Use another</span>
		<span class="spacer"></span>
		<button class="btn small" onclick={browse}>Folder…</button>
	</label>
	{#if choice === 'use'}
		<div class="list">
			{#if loading}
				<div class="trow"><span class="spinner"></span></div>
			{/if}
			{#each launchers as launcher (launcher)}
				<div class="band">{launcher}</div>
				{#each detected.filter((d) => d.launcher === launcher) as found (found.gameDir)}
					<button
						class="trow"
						class:selected={picked === found.gameDir}
						onclick={() => (picked = found.gameDir)}
					>
						<span class="name">{found.name}</span>
						<span class="small"
							>{found.minecraft ?? '?'} · {loaderName(found.loader ?? 'vanilla')}</span
						>
						<span class="spacer"></span>
						{#if compatible(found)}
							<span class="tag added">✓ compatible</span>
						{:else}
							<span class="tag removed">✗ incompatible</span>
						{/if}
					</button>
				{/each}
			{/each}
		</div>
	{/if}
	{#snippet footer()}
		<button class="btn" onclick={() => close()}>Cancel</button>
		<button class="btn primary" disabled={choice === 'use' && !picked} onclick={submit}>Link</button
		>
	{/snippet}
</Dialog>

<style>
	.option {
		display: flex;
		align-items: center;
		gap: 10px;
		min-height: 36px;
		padding: 0 12px;
		background: var(--band);
	}

	.option.on {
		background: var(--selected);
	}

	.option :global(.grow) {
		flex: 1;
	}

	.list {
		max-height: 260px;
		overflow: auto;
		border: 1px solid var(--line);
	}

	.list .trow {
		width: 100%;
		text-align: left;
	}
</style>

<script lang="ts">
	import { onMount } from 'svelte'

	import { toasts } from '@/lib/state/toasts.svelte'
	import { loaderVersions, minecraftVersions } from '@/lib/tauri/catalog'
	import { loaderName, loaderVersionOptions } from '@/lib/util/format'
	import Dialog from '@/ui/Dialog.svelte'
	import Segmented from '@/ui/Segmented.svelte'
	import Select from '@/ui/Select.svelte'
	import TextField from '@/ui/TextField.svelte'

	import FolderField from './FolderField.svelte'

	interface Result {
		name: string
		minecraft: string
		loader: string
		loaderVersion: string | null
		parent: string
		git: boolean
		prism: boolean
	}

	interface Props {
		close: (result?: Result) => void
	}

	let { close }: Props = $props()

	let name = $state('')
	let minecraft = $state('')
	let loader = $state('fabric')
	let loaderVersion = $state('')
	let parent = $state('')
	let useGit = $state(true)
	let prism = $state(false)
	let versions = $state<string[]>([])
	let loaderOptions = $state<string[]>([])
	let loaderState = $state<'loading' | 'ready' | 'failed'>('loading')

	const noLoader = $derived(
		loader !== 'vanilla' && loaderState === 'ready' && loaderOptions.length === 0,
	)
	const canCreate = $derived(!!name.trim() && !!minecraft && !!parent && !noLoader)

	onMount(() => {
		void minecraftVersions(false)
			.then((list) => {
				versions = list
				minecraft ||= list[0] ?? '1.21.1'
			})
			.catch((error) => toasts.error(error))
	})

	$effect(() => {
		const mc = minecraft
		const kind = loader
		loaderOptions = []
		loaderVersion = ''
		loaderState = 'loading'
		if (!mc || kind === 'vanilla') return
		void loaderVersions(kind, mc)
			.then((found) => {
				if (minecraft !== mc || loader !== kind) return
				loaderOptions = found.versions.slice(0, 50)
				loaderVersion = found.versions[0] ?? ''
				loaderState = 'ready'
			})
			.catch((error) => {
				if (minecraft === mc && loader === kind) loaderState = 'failed'
				toasts.error(error)
			})
	})

	function submit(): void {
		if (!canCreate) return
		close({
			name: name.trim(),
			minecraft,
			loader,
			loaderVersion: loader === 'vanilla' || !loaderVersion ? null : loaderVersion,
			parent,
			git: useGit,
			prism,
		})
	}
</script>

<Dialog title="New pack" onclose={() => close()}>
	<div class="form-field">
		<span class="label">Name</span>
		<TextField
			bind:value={name}
			placeholder="Skyward"
			autofocus
			onkeydown={(e) => e.key === 'Enter' && submit()}
		/>
	</div>
	<div class="form-field">
		<span class="label">Minecraft</span>
		<Select options={versions.map((v) => ({ value: v, label: v }))} bind:value={minecraft} mono />
	</div>
	<div class="form-field">
		<span class="label">Loader</span>
		<Segmented
			options={[
				{ value: 'fabric', label: 'Fabric' },
				{ value: 'quilt', label: 'Quilt' },
				{ value: 'neoforge', label: 'NeoForge' },
				{ value: 'forge', label: 'Forge' },
				{ value: 'vanilla', label: 'Vanilla' },
			]}
			bind:value={loader}
		/>
	</div>
	{#if loader !== 'vanilla'}
		<div class="form-field">
			<span class="label">Loader version</span>
			<Select
				options={loaderVersionOptions(loaderOptions, loaderVersion)}
				bind:value={loaderVersion}
				placeholder={loaderState === 'loading'
					? 'Loading…'
					: loaderState === 'failed'
						? "Couldn't load versions"
						: `No ${loaderName(loader)} for Minecraft ${minecraft}`}
				mono
			/>
		</div>
	{/if}
	<div class="form-field">
		<span class="label">Folder</span>
		<FolderField bind:value={parent} title="Choose where the pack folder goes" />
	</div>
	<label class="check"><input type="checkbox" bind:checked={useGit} />Use Git</label>
	<label class="check"><input type="checkbox" bind:checked={prism} />Create a Prism instance</label>
	{#snippet footer()}
		<button class="btn" onclick={() => close()}>Cancel</button>
		<button class="btn primary" disabled={!canCreate} onclick={submit}>Create pack</button>
	{/snippet}
</Dialog>

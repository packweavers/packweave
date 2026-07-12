<script lang="ts">
	import { Link2, X } from '@lucide/svelte'
	import Modal from './ui/Modal.svelte'
	import ButtonStyled from './ui/ButtonStyled.svelte'
	import Select from './ui/Select.svelte'
	import InstancePicker from './InstancePicker.svelte'
	import { api } from '../api'
	import { store } from '../lib/store.svelte'
	import { LOADERS, type DetectedInstance } from '../types'
	import { basename, loaderLabel } from '../util'

	let { onclose }: { onclose?: () => void } = $props()

	let name = $state('My Modpack')
	let minecraft = $state('')
	let loader = $state<string>('fabric')
	let loaderVersion = $state('')
	let mcVersions = $state<string[]>([])
	let loaderVersions = $state<string[]>([])
	let instance = $state<DetectedInstance | null>(null)
	let showPicker = $state(false)
	let pendingLoaderVersion = $state<string | null>(null)

	function onPick(picked: DetectedInstance) {
		instance = picked
		showPicker = false
		if (picked.loader) loader = picked.loader
		if (picked.loaderVersion) {
			loaderVersion = picked.loaderVersion
			pendingLoaderVersion = picked.loaderVersion
		}
		if (picked.minecraft) minecraft = picked.minecraft
		if (name === 'My Modpack' && picked.name) name = picked.name
	}

	api
		.getMinecraftVersions()
		.then((list) => {
			mcVersions = list
			if (!minecraft) minecraft = list[0] ?? ''
		})
		.catch(() => {})

	$effect(() => {
		const l = loader
		const mc = minecraft
		if (l === 'vanilla') {
			loaderVersions = []
			loaderVersion = ''
			return
		}
		api
			.getLoaderVersions(l, mc)
			.then((list) => {
				loaderVersions = list
				if (pendingLoaderVersion) {
					loaderVersion = pendingLoaderVersion
					pendingLoaderVersion = null
				} else {
					loaderVersion = list[0] ?? ''
				}
			})
			.catch(() => {
				loaderVersions = []
				loaderVersion = ''
			})
	})

	async function create() {
		const ok = await store.createPack(
			name.trim(),
			minecraft.trim(),
			loader,
			loaderVersion.trim() || null,
			instance?.gameDir ?? null,
		)
		if (!ok) return
		onclose?.()
	}
</script>

<Modal title="New pack" onclose={() => onclose?.()}>
	<label class="flex flex-col gap-[0.35rem] mb-[0.85rem]">
		<span class="text-[0.78rem] text-secondary font-[550]">Pack name</span>
		<!-- svelte-ignore a11y_autofocus -->
		<input
			bind:value={name}
			autofocus
			class="bg-bg border border-divider text-contrast rounded-md px-[0.7rem] py-[0.55rem] text-sm outline-none focus:border-brand"
		/>
	</label>

	<div class="grid grid-cols-2 gap-3">
		<div class="flex flex-col gap-[0.35rem] mb-[0.85rem]">
			<span class="text-[0.78rem] text-secondary font-[550]">Minecraft version</span>
			<Select bind:value={minecraft} options={mcVersions} allowCustom placeholder="1.20.1" />
		</div>
		<div class="flex flex-col gap-[0.35rem] mb-[0.85rem]">
			<span class="text-[0.78rem] text-secondary font-[550]">Loader</span>
			<Select bind:value={loader} options={LOADERS} display={loaderLabel} searchable={false} />
		</div>
	</div>

	{#if loader !== 'vanilla'}
		<div class="flex flex-col gap-[0.35rem] mb-[0.85rem]">
			<span class="text-[0.78rem] text-secondary font-[550]">Loader version</span>
			<Select bind:value={loaderVersion} options={loaderVersions} allowCustom placeholder="latest" />
		</div>
	{/if}

	<div class="flex flex-col gap-[0.35rem] mb-[0.85rem]">
		<span class="text-[0.78rem] text-secondary font-[550]">Instance <em class="not-italic opacity-70">optional</em></span>
		{#if !instance}
			<button
				class="flex items-center gap-[0.4rem] w-full bg-bg border border-dashed border-divider text-secondary rounded-md px-[0.7rem] py-[0.55rem] text-[0.85rem] cursor-pointer hover:border-brand hover:text-body"
				onclick={() => (showPicker = true)}
			>
				<Link2 size={15} />
				Link an instance…
			</button>
		{:else}
			<div
				class="flex items-center justify-between gap-2 bg-bg border border-divider rounded-md px-[0.6rem] py-[0.45rem]"
			>
				<div>
					<div class="font-semibold text-contrast text-[0.85rem]">{instance.name}</div>
					<div class="text-[0.7rem] text-secondary">
						{basename(instance.gameDir)} · {instance.launcher}
					</div>
				</div>
				<button
					class="bg-transparent border-none text-secondary cursor-pointer grid place-items-center p-[0.2rem] hover:text-red"
					onclick={() => (instance = null)}
				>
					<X size={14} />
				</button>
			</div>
		{/if}
	</div>

	<div class="flex justify-end gap-[0.6rem] mt-2">
		<ButtonStyled type="transparent" disabled={store.busy} onclick={() => onclose?.()}>
			Cancel
		</ButtonStyled>
		<ButtonStyled
			color="brand"
			disabled={store.busy || !name.trim() || !minecraft.trim()}
			onclick={create}
		>
			Choose folder & create
		</ButtonStyled>
	</div>
</Modal>

{#if showPicker}
	<InstancePicker onpick={onPick} onclose={() => (showPicker = false)} />
{/if}

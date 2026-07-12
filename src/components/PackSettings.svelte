<script lang="ts">
	import { ImagePlus } from '@lucide/svelte'
	import ButtonStyled from './ui/ButtonStyled.svelte'
	import Select from './ui/Select.svelte'
	import { api, pickImage } from '../api'
	import { store } from '../lib/store.svelte'
	import { LOADERS } from '../types'
	import { loaderLabel } from '../util'

	let { ondone }: { ondone?: () => void } = $props()

	let icon = $state<string | null>(null)
	async function loadIcon() {
		icon = store.pack ? await api.packIcon(store.pack.dir).catch(() => null) : null
	}
	loadIcon()
	async function changeIcon() {
		const src = await pickImage()
		if (!src) return
		await store.applyPackIcon(src)
		await loadIcon()
	}
	async function removeIcon() {
		await store.applyPackIcon(null)
		await loadIcon()
	}

	let version = $state(store.pack?.manifest.version ?? '1.0.0')
	let minecraft = $state(store.pack?.manifest.minecraft ?? '')
	let loader = $state(store.pack?.manifest.loader ?? 'fabric')
	let loaderVersion = $state(store.pack?.manifest.loaderVersion ?? '')
	let channel = $state(store.pack?.manifest.channel ?? 'release')
	let mcVersions = $state<string[]>([])
	let loaderVersions = $state<string[]>([])

	const CHANNELS = ['release', 'beta', 'alpha']
	const channelLabel = (c: string) =>
		({
			release: 'Stable releases only',
			beta: 'Allow betas',
			alpha: 'Allow betas & alphas',
		})[c] ?? c

	api
		.getMinecraftVersions()
		.then((l) => (mcVersions = l))
		.catch(() => {})

	let lastDir: string | null | undefined = store.pack?.dir
	$effect(() => {
		const dir = store.pack?.dir
		if (dir !== lastDir) {
			lastDir = dir
			version = store.pack?.manifest.version ?? '1.0.0'
			minecraft = store.pack?.manifest.minecraft ?? ''
			loader = store.pack?.manifest.loader ?? 'fabric'
			loaderVersion = store.pack?.manifest.loaderVersion ?? ''
			channel = store.pack?.manifest.channel ?? 'release'
			loadIcon()
		}
	})

	$effect(() => {
		const l = loader
		const mc = minecraft
		api
			.getLoaderVersions(l, mc)
			.then((vs) => (loaderVersions = vs))
			.catch(() => (loaderVersions = []))
	})

	async function apply() {
		await store.setPackVersion(version.trim())
		await store.updateSettings(minecraft.trim(), loader, loaderVersion.trim() || null, channel)
		ondone?.()
	}
</script>

<div class="w-[280px] p-[0.4rem]">
	<div class="font-semibold text-contrast">Pack settings</div>

	<div class="flex items-center gap-[0.7rem] mt-[0.7rem] mb-[0.9rem]">
		<div
			class="w-[52px] h-[52px] rounded-md overflow-hidden shrink-0 border border-divider bg-bg {icon
				? ''
				: 'grid place-items-center text-secondary border-dashed'}"
		>
			{#if icon}
				<img src={icon} alt="" class="w-full h-full object-cover block" />
			{:else}
				<ImagePlus size={20} />
			{/if}
		</div>
		<div class="flex flex-col gap-[0.35rem] items-start">
			<ButtonStyled size="small" type="outlined" disabled={store.busy} onclick={changeIcon}>
				{icon ? 'Change icon' : 'Add icon'}
			</ButtonStyled>
			{#if icon}
				<ButtonStyled
					size="small"
					type="transparent"
					color="red"
					disabled={store.busy}
					onclick={removeIcon}
				>
					Remove
				</ButtonStyled>
			{/if}
		</div>
	</div>

	<label class="flex flex-col gap-[0.3rem] mb-[0.6rem]">
		<span class="text-[0.72rem] text-secondary font-[550]">Version</span>
		<input
			bind:value={version}
			placeholder="1.0.0"
			spellcheck="false"
			class="bg-bg border border-divider text-contrast rounded-sm px-[0.55rem] py-[0.4rem] text-[0.82rem] outline-none focus:border-brand"
		/>
	</label>
	<div class="flex flex-col gap-[0.3rem] mb-[0.6rem]">
		<span class="text-[0.72rem] text-secondary font-[550]">Minecraft version</span>
		<Select bind:value={minecraft} options={mcVersions} allowCustom placeholder="1.20.1" />
	</div>
	<div class="flex flex-col gap-[0.3rem] mb-[0.6rem]">
		<span class="text-[0.72rem] text-secondary font-[550]">Loader</span>
		<Select bind:value={loader} options={LOADERS} display={loaderLabel} searchable={false} />
	</div>
	{#if loader !== 'vanilla'}
		<div class="flex flex-col gap-[0.3rem] mb-[0.6rem]">
			<span class="text-[0.72rem] text-secondary font-[550]">Loader version</span>
			<Select bind:value={loaderVersion} options={loaderVersions} allowCustom placeholder="latest" />
		</div>
	{/if}

	<div class="flex flex-col gap-[0.3rem] mb-[0.2rem]">
		<span class="text-[0.72rem] text-secondary font-[550]">Update channel</span>
		<Select bind:value={channel} options={CHANNELS} display={channelLabel} searchable={false} />
		<span class="text-[0.68rem] text-secondary leading-[1.4]">
			Which versions “always latest” mods may update to. Pinned mods are unaffected.
		</span>
	</div>

	<div class="flex justify-end gap-2 mt-[0.7rem]">
		<ButtonStyled size="small" type="transparent" disabled={store.busy} onclick={() => ondone?.()}>
			Cancel
		</ButtonStyled>
		<ButtonStyled
			size="small"
			color="brand"
			disabled={store.busy || !minecraft.trim()}
			onclick={apply}
		>
			Apply
		</ButtonStyled>
	</div>
</div>

<script lang="ts">
	import { onMount } from 'svelte'
	import { FolderOpen, Link2 } from '@lucide/svelte'
	import Modal from './ui/Modal.svelte'
	import ButtonStyled from './ui/ButtonStyled.svelte'
	import { api, pickFolder } from '../api'
	import { contextMenu } from '../lib/contextmenu.svelte'
	import type { DetectedInstance } from '../types'
	import { basename, capitalize, loaderLabel } from '../util'

	let {
		onpick,
		onclose,
	}: {
		onpick?: (instance: DetectedInstance) => void
		onclose?: () => void
	} = $props()

	let instances = $state<DetectedInstance[]>([])
	let loading = $state(true)

	onMount(async () => {
		try {
			instances = await api.detectInstances()
		} catch {
			instances = []
		}
		loading = false
	})

	function sourceLabel(src: string | null): string {
		if (src === 'modrinth') return 'Modrinth'
		if (src === 'curseforge') return 'CurseForge'
		return src ? capitalize(src) : ''
	}

	function kindLabel(inst: DetectedInstance): string | null {
		if (inst.kind === 'server') return `${sourceLabel(inst.source)} server`.trim()
		if (inst.kind === 'modpack') return `${sourceLabel(inst.source)} modpack`.trim()
		return null
	}

	async function browse() {
		const dir = await pickFolder('Choose an instance folder')
		if (!dir) return
		try {
			onpick?.(await api.resolveInstanceFolder(dir))
		} catch {
			onpick?.({
				launcher: 'Folder',
				name: basename(dir),
				gameDir: dir,
				minecraft: null,
				loader: null,
				loaderVersion: null,
				kind: 'local',
				source: null,
				packName: null,
				packVersion: null,
				iconPath: null,
			})
		}
	}
</script>

<Modal title="Link an instance" onclose={() => onclose?.()}>
	{#if loading}
		<div class="text-center text-secondary p-6 text-regular">Scanning launchers…</div>
	{:else if instances.length === 0}
		<div class="text-center text-secondary p-6 text-regular">
			No instances detected. Browse to any Minecraft folder below.
		</div>
	{:else}
		<ul class="list-none m-0 p-0 max-h-[44vh] overflow-y-auto">
			{#each instances as inst, i (i)}
				<li
					class="px-3 py-2.5 rounded-md border border-divider mb-md cursor-pointer hover:border-brand hover:bg-bg-raised"
					onclick={() => onpick?.(inst)}
					use:contextMenu={() => [
						{ label: 'Link instance', icon: Link2, onSelect: () => onpick?.(inst) },
					]}
				>
					<div class="font-semibold text-contrast flex items-center gap-2">
						{inst.name}
						{#if inst.minecraft || inst.loader}
							<span class="inline-flex gap-sm">
								{#if inst.minecraft}
									<span
										class="text-micro font-semibold bg-button-bg text-secondary px-md py-hair rounded-sm"
										>{inst.minecraft}</span
									>
								{/if}
								{#if inst.loader}
									<span
										class="text-micro font-semibold bg-button-bg text-secondary px-md py-hair rounded-sm"
										>{loaderLabel(inst.loader)}</span
									>
								{/if}
							</span>
						{/if}
					</div>
					<div class="flex items-center gap-1.5 mt-2xs">
						<span class="text-tiny text-brand">{inst.launcher}</span>
						{#if kindLabel(inst)}
							<span
								class="text-micro font-semibold uppercase tracking-[0.04em] bg-brand/15 text-brand px-md py-hair rounded-sm"
								>{kindLabel(inst)}</span
							>
							{#if inst.packName}
								<span class="text-micro text-secondary truncate">
									{inst.packName}{inst.packVersion ? ` · ${inst.packVersion}` : ''}
								</span>
							{/if}
						{/if}
					</div>
					<div
						class="text-tiny text-secondary whitespace-nowrap overflow-hidden text-ellipsis mt-2xs"
					>
						{inst.gameDir}
					</div>
				</li>
			{/each}
		</ul>
	{/if}

	<div class="flex justify-center mt-3 pt-3 border-t border-divider">
		<ButtonStyled type="outlined" onclick={browse}>
			<FolderOpen size={15} />
			Browse for a folder…
		</ButtonStyled>
	</div>
</Modal>

<script lang="ts">
	import { Check, FileBox, Pin } from '@lucide/svelte'
	import Avatar from './ui/Avatar.svelte'
	import ButtonStyled from './ui/ButtonStyled.svelte'
	import Modal from './ui/Modal.svelte'
	import { store } from '../lib/store.svelte'
	import { providerLabel } from '../lib/mods'
	import type { DroppedFile } from '../types'

	let { files, onclose }: { files: DroppedFile[]; onclose: () => void } = $props()

	let excluded = $state(new Set<string>())
	const isPicked = (f: DroppedFile) => !f.alreadyInPack && !excluded.has(f.path)
	function togglePick(f: DroppedFile) {
		if (excluded.has(f.path)) excluded.delete(f.path)
		else excluded.add(f.path)
		excluded = new Set(excluded)
	}
	const chosen = $derived(files.filter(isPicked))
	const matchedCount = $derived(chosen.filter((f) => f.matched).length)
	const bundledCount = $derived(chosen.filter((f) => !f.matched).length)

	async function confirm() {
		const items = chosen
		onclose()
		await store.addDropped(items)
	}

	const dirLabel = (t: string) =>
		t === 'resourcepack' ? 'resourcepacks' : t === 'shader' ? 'shaderpacks' : 'mods'
</script>

<Modal title="Add dropped files" {onclose}>
	<div class="flex flex-col gap-[0.2rem] max-h-[340px] overflow-y-auto mb-3">
		{#each files as f (f.path)}
			<label
				class={`flex items-center gap-[0.6rem] px-[0.4rem] py-[0.45rem] rounded-md ${
					f.alreadyInPack ? 'opacity-50' : 'cursor-pointer hover:bg-bg-raised'
				}`}
			>
				<input
					type="checkbox"
					checked={isPicked(f)}
					disabled={f.alreadyInPack}
					onchange={() => togglePick(f)}
					class="w-4 h-4 accent-brand cursor-pointer shrink-0 disabled:cursor-default"
				/>
				{#if f.matched}
					<Avatar src={f.matched.iconUrl ?? null} alt={f.matched.name} size={32} />
				{:else}
					<span class="grid place-items-center w-8 h-8 rounded-md bg-bg-inset text-secondary shrink-0">
						<FileBox size={16} />
					</span>
				{/if}
				<div class="flex-1 min-w-0">
					<div class="font-medium text-contrast whitespace-nowrap overflow-hidden text-ellipsis">
						{f.matched?.name ?? f.filename}
					</div>
					<div class="text-[0.72rem] text-secondary mt-[0.1rem] flex items-center gap-[0.35rem] flex-wrap">
						{#if f.alreadyInPack}
							<span class="inline-flex items-center gap-1 text-green"><Check size={12} /> already in the pack</span>
						{:else if f.matched}
							<span>{providerLabel(f.matched.provider)}</span>
							<span class="opacity-50">·</span>
							<span class="inline-flex items-center gap-1 font-mono"
								><Pin size={11} /> {f.matched.versionNumber}</span
							>
							<span class="opacity-50">·</span>
							<span>{f.projectType}</span>
						{:else}
							<span>not on Modrinth or CurseForge, bundled into overrides/{dirLabel(f.projectType)}</span>
						{/if}
					</div>
				</div>
			</label>
		{/each}
	</div>

	<p class="text-[0.76rem] text-secondary mb-4 leading-[1.5]">
		Matched files are added as proper sources, pinned to the exact version you dropped.
	</p>

	<div class="flex justify-end items-center gap-[0.6rem]">
		<ButtonStyled type="transparent" disabled={store.busy} onclick={onclose}>Cancel</ButtonStyled>
		<ButtonStyled color="brand" disabled={!chosen.length || store.busy} onclick={confirm}>
			{#if matchedCount && bundledCount}
				Add {matchedCount} · bundle {bundledCount}
			{:else if bundledCount}
				Bundle {bundledCount}
			{:else}
				Add {matchedCount}
			{/if}
		</ButtonStyled>
	</div>
</Modal>

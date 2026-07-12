<script lang="ts">
	import { ChevronDown, FileBox, Trash2 } from '@lucide/svelte'
	import { tooltip } from '../../lib/tooltip'
	import { store } from '../../lib/store.svelte'
	import { removeUnpublished, unpubMissingDeps } from '../../lib/mods'
	import type { LocalContent } from '../../types'
	import { formatBytes } from '../../util'

	let { items }: { items: LocalContent[] } = $props()

	let open = $state(false)
	let confirmPath = $state<string | null>(null)
</script>

<div class="mt-6 border-t border-divider pt-2">
	<button
		class="flex items-center gap-[0.4rem] bg-transparent border-none text-secondary text-[0.74rem] cursor-pointer px-[0.2rem] py-[0.3rem] hover:text-body"
		onclick={() => (open = !open)}
	>
		<FileBox size={13} />
		{items.length} unpublished
		{items.length === 1 ? 'file' : 'files'}
		<ChevronDown
			size={13}
			class={`transition-transform duration-[0.12s] ${open ? '' : '-rotate-90'}`}
		/>
	</button>
	{#if open}
		<div class="flex flex-col gap-[0.4rem] my-2">
			{#each items as u (u.relPath)}
				<div class="group flex items-center gap-[0.7rem] px-[0.6rem] py-2 rounded-md bg-bg-inset">
					<FileBox size={16} class="text-secondary shrink-0" />
					<div class="flex-1 min-w-0">
						<div
							class="text-[0.82rem] font-medium text-contrast whitespace-nowrap overflow-hidden text-ellipsis"
						>
							{u.meta?.name ?? u.filename}
							{#if u.meta?.version}<span class="font-mono text-[0.72rem] text-secondary ml-1"
									>{u.meta.version}</span
								>{/if}
						</div>
						<div class="text-[0.74rem] text-secondary mt-[0.1rem] flex items-center gap-[0.4rem] flex-wrap">
							<span>{formatBytes(u.size)}</span>
							{#if u.meta?.loaders?.length}<span>· {u.meta.loaders.join(', ')}</span>{/if}
							{#if u.meta?.packFormat != null}<span>· pack format {u.meta.packFormat}</span>{/if}
							{#if u.meta?.name}<span class="font-mono opacity-80">· {u.filename}</span>{/if}
						</div>
						{#if unpubMissingDeps(u).length}
							<div class="text-[0.72rem] text-orange mt-[0.15rem]">
								Needs {unpubMissingDeps(u).join(', ')}
							</div>
						{/if}
					</div>
					{#if confirmPath === u.relPath}
						<span
							class="inline-flex items-center gap-1"
							role="presentation"
							onclick={(e) => e.stopPropagation()}
						>
							<button
								class="bg-red text-white text-[0.72rem] font-semibold px-2 py-[0.25rem] rounded-sm cursor-pointer disabled:opacity-60"
								disabled={store.busy}
								onclick={() => {
									confirmPath = null
									removeUnpublished(u.relPath)
								}}
							>
								Delete
							</button>
							<button
								class="bg-button-bg text-body text-[0.72rem] px-2 py-[0.25rem] rounded-sm cursor-pointer hover:text-contrast"
								onclick={() => (confirmPath = null)}>No</button
							>
						</span>
					{:else}
						<button
							class="grid place-items-center w-[1.8rem] h-[1.8rem] rounded-sm border-none bg-transparent text-secondary cursor-pointer opacity-100 shrink-0 hover:bg-button-bg hover:text-red disabled:opacity-60"
							aria-label="Remove file"
							use:tooltip={'Remove file'}
							disabled={store.busy}
							onclick={() => (confirmPath = u.relPath)}
						>
							<Trash2 size={14} />
						</button>
					{/if}
				</div>
			{/each}
		</div>
	{/if}
</div>

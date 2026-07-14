<script lang="ts">
	import { Boxes, FolderOpen, Clock, X, Plus } from '@lucide/svelte'
	import ButtonStyled from './ui/ButtonStyled.svelte'
	import WindowControls from './WindowControls.svelte'
	import { store } from '../lib/store.svelte'
	import { contextMenu } from '../lib/contextmenu.svelte'
	import { tooltip } from '../lib/tooltip'
	import { fromNowMs, isMac, loaderLabel } from '../util'

	let { onnew }: { onnew?: () => void } = $props()

	const recents = $derived(store.recents)
</script>

<div
	class="h-full overflow-y-auto flex flex-col bg-bg [background:radial-gradient(900px_460px_at_50%_0%,var(--color-brand-highlight),transparent_70%),var(--color-bg)]"
>
	<div
		class="h-[var(--titlebar-height)] shrink-0 flex items-stretch"
		data-tauri-drag-region
	>
		<div class="flex-1" data-tauri-drag-region></div>
		{#if !isMac}<WindowControls />{/if}
	</div>
	<div
		class="flex-1 w-full max-w-[460px] mx-auto flex flex-col items-center text-center px-6 pt-[3vh] pb-12"
	>
		<div class="w-[46px] h-[46px] rounded-[11px] bg-brand rotate-45 mt-2 mb-[1.6rem]"></div>
		<h1 class="text-[1.7rem]">packweave</h1>
		<p class="text-secondary text-[0.9rem] mt-[0.3rem] mb-[2rem]">
			A Minecraft modpack IDE.
		</p>
		<div class="mb-[2.4rem]">
			<ButtonStyled color="brand" size="large" disabled={store.busy} onclick={() => onnew?.()}>
				<Plus size={16} /> Get Started
			</ButtonStyled>
		</div>

		{#if recents.length}
			<div class="w-full text-left">
				<div
					class="flex items-center gap-[0.4rem] text-[0.72rem] uppercase tracking-[0.05em] text-secondary font-[650] mb-2 pl-[0.2rem]"
				>
					<Clock size={13} /> Recent
				</div>
				<ul class="list-none m-0 p-0 flex flex-col gap-[0.3rem]">
					{#each recents as r (r.dir)}
						<li>
							<button
								class="group relative flex items-center gap-[0.65rem] w-full text-left bg-bg-raised border border-divider rounded-md px-[0.65rem] py-[0.55rem] cursor-pointer hover:enabled:border-divider-dark disabled:opacity-60 disabled:cursor-default"
								disabled={store.busy}
								onclick={() => store.openRecent(r.dir)}
								use:contextMenu={() => [
									{ label: 'Open', icon: FolderOpen, onSelect: () => store.openRecent(r.dir) },
									{ label: 'Remove', icon: X, danger: true, onSelect: () => store.removeRecent(r.dir) },
								]}
							>
								<span
									class="w-8 h-8 grid place-items-center rounded-sm bg-bg-inset text-secondary shrink-0"
								>
									<Boxes size={16} />
								</span>
								<span class="flex-1 min-w-0">
									<span class="block font-semibold text-contrast text-[0.88rem]">{r.name}</span>
									<span class="block text-[0.72rem] text-secondary whitespace-nowrap overflow-hidden text-ellipsis">
										{r.minecraft} · {loaderLabel(r.loader)} · {fromNowMs(r.lastOpened)}
									</span>
								</span>
								<span
									class="grid place-items-center w-6 h-6 rounded-sm text-secondary opacity-0 group-hover:opacity-100 hover:bg-button-bg hover:text-red"
									role="button"
									tabindex="0"
									use:tooltip={'Remove'}
									onclick={(e) => {
										e.stopPropagation()
										store.removeRecent(r.dir)
									}}
									onkeydown={(e) => {
										if (e.key === 'Enter') {
											e.stopPropagation()
											store.removeRecent(r.dir)
										}
									}}
								>
									<X size={13} />
								</span>
							</button>
						</li>
					{/each}
				</ul>
			</div>
		{/if}
	</div>
</div>

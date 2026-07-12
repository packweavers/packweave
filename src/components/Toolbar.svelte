<script lang="ts">
	import {
		ArrowLeft,
		ArrowRight,
		Layers,
		Folder,
		Plug,
		GitBranch,
		Package,
		Plus,
		ChevronDown,
		FolderOpen,
		Link2,
		Unlink,
		Settings,
		CircleCheck,
		X,
	} from '@lucide/svelte'
	import Dropdown from './ui/Dropdown.svelte'
	import PackSettings from './PackSettings.svelte'
	import WindowControls from './WindowControls.svelte'
	import ButtonStyled from './ui/ButtonStyled.svelte'
	import { tooltip } from '../lib/tooltip'
	import { store } from '../lib/store.svelte'
	import type { Validation } from '../types'
	import { isMac, loaderLabel } from '../util'

	let {
		onadd,
		onexport,
		onsettings,
		onlink,
	}: { onadd?: () => void; onexport?: () => void; onsettings?: () => void; onlink?: () => void } =
		$props()

	const manifest = $derived(store.pack?.manifest)
	const syncCount = $derived(store.syncChangeCount)
	const gitCount = $derived(store.git?.changes.length ?? 0)
	const otherRecents = $derived(
		store.recents.filter((r) => r.dir !== store.pack?.dir).slice(0, 6),
	)
	const busy = $derived(store.busy)

	const health = $derived.by(() => {
		const h = store.health
		if (!h) return { kind: 'muted', label: '-' }
		if (h.errors > 0)
			return { kind: 'error', label: `${h.errors} ${h.errors === 1 ? 'problem' : 'problems'}` }
		if (h.warnings > 0)
			return { kind: 'warn', label: `${h.warnings} ${h.warnings === 1 ? 'warning' : 'warnings'}` }
		return { kind: 'ok', label: 'Healthy' }
	})

	const problems = $derived(
		store.validations.filter((v) => v.severity === 'error' || v.severity === 'warning'),
	)

	const healthCls: Record<string, string> = {
		ok: 'text-secondary',
		warn: 'text-orange',
		error: 'text-red',
		muted: 'text-secondary',
	}
	const dotCls: Record<string, string> = {
		ok: 'bg-green',
		warn: 'bg-orange',
		error: 'bg-red',
		muted: 'bg-secondary',
	}

	function goToProblem(_p: Validation) {
		store.setView('content')
	}
</script>

<header
	class="relative flex items-center gap-2 h-12 pr-[0.3rem] {isMac
		? 'pl-20'
		: 'pl-[0.6rem]'} bg-chrome [backdrop-filter:saturate(180%)_blur(24px)] border-b border-divider shrink-0"
	data-tauri-drag-region
>
	<div class="flex gap-[0.1rem]">
		<button
			class="grid place-items-center w-[1.8rem] h-[1.7rem] border-0 bg-transparent text-secondary rounded-sm cursor-pointer enabled:hover:bg-button-bg enabled:hover:text-contrast disabled:opacity-30 disabled:cursor-default"
			disabled={!store.canNavBack}
			use:tooltip={'Back'}
			onclick={() => store.navBack()}
		>
			<ArrowLeft size={16} />
		</button>
		<button
			class="grid place-items-center w-[1.8rem] h-[1.7rem] border-0 bg-transparent text-secondary rounded-sm cursor-pointer enabled:hover:bg-button-bg enabled:hover:text-contrast disabled:opacity-30 disabled:cursor-default"
			disabled={!store.canNavForward}
			use:tooltip={'Forward'}
			onclick={() => store.navForward()}
		>
			<ArrowRight size={16} />
		</button>
	</div>

	<Dropdown placement="bottom-start">
		{#snippet trigger()}
			<button
				class="inline-flex items-center gap-[0.3rem] bg-transparent border-0 text-contrast text-[0.85rem] font-semibold px-[0.45rem] py-[0.3rem] rounded-md cursor-pointer max-w-[180px] hover:bg-button-bg"
			>
				<span class="whitespace-nowrap overflow-hidden text-ellipsis">{manifest?.name}</span>
				<ChevronDown size={13} class="text-secondary shrink-0" />
			</button>
		{/snippet}
		{#snippet content(hide)}
			<div class="min-w-[210px] flex flex-col gap-px">
				{#if otherRecents.length}
					<div
						class="text-[0.66rem] uppercase tracking-[0.05em] text-secondary font-[650] pt-[0.35rem] px-[0.6rem] pb-[0.25rem]"
					>
						Switch pack
					</div>
					{#each otherRecents as r (r.dir)}
						<button
							class="flex items-center gap-[0.55rem] bg-transparent border-0 text-body text-[0.85rem] text-left px-[0.6rem] py-[0.45rem] rounded-sm cursor-pointer w-full hover:bg-button-bg hover:text-contrast"
							onclick={() => {
								hide()
								store.openRecent(r.dir)
							}}
						>
							<Layers size={15} />
							<span class="flex flex-col"
								>{r.name}<span class="text-[0.7rem] text-secondary"
									>{r.minecraft} · {loaderLabel(r.loader)}</span
								></span
							>
						</button>
					{/each}
					<div class="h-px bg-divider my-1"></div>
				{/if}
				<button
					class="flex items-center gap-[0.55rem] bg-transparent border-0 text-body text-[0.85rem] text-left px-[0.6rem] py-[0.45rem] rounded-sm cursor-pointer w-full hover:bg-button-bg hover:text-contrast"
					onclick={() => {
						hide()
						store.openPack()
					}}
				>
					<FolderOpen size={15} /><span>Open folder…</span>
				</button>
				{#if !store.bound}
					<button
						class="flex items-center gap-[0.55rem] bg-transparent border-0 text-body text-[0.85rem] text-left px-[0.6rem] py-[0.45rem] rounded-sm cursor-pointer w-full hover:bg-button-bg hover:text-contrast"
						onclick={() => {
							hide()
							onlink?.()
						}}
					>
						<Link2 size={15} /><span>Link instance…</span>
					</button>
				{:else}
					<button
						class="flex items-center gap-[0.55rem] bg-transparent border-0 text-body text-[0.85rem] text-left px-[0.6rem] py-[0.45rem] rounded-sm cursor-pointer w-full hover:bg-button-bg hover:text-contrast"
						onclick={() => {
							hide()
							store.unlinkInstance()
						}}
					>
						<Unlink size={15} /><span>Unlink instance</span>
					</button>
				{/if}
				{#if !store.git?.isRepo}
					<button
						class="flex items-center gap-[0.55rem] bg-transparent border-0 text-body text-[0.85rem] text-left px-[0.6rem] py-[0.45rem] rounded-sm cursor-pointer w-full hover:bg-button-bg hover:text-contrast"
						onclick={() => {
							hide()
							store.gitInit()
						}}
					>
						<GitBranch size={15} /><span>Initialize Git…</span>
					</button>
				{/if}
				<button
					class="flex items-center gap-[0.55rem] bg-transparent border-0 text-body text-[0.85rem] text-left px-[0.6rem] py-[0.45rem] rounded-sm cursor-pointer w-full hover:bg-button-bg hover:text-contrast"
					onclick={() => {
						hide()
						onsettings?.()
					}}
				>
					<Settings size={15} /><span>Settings…</span>
				</button>
				<div class="h-px bg-divider my-1"></div>
				<button
					class="flex items-center gap-[0.55rem] bg-transparent border-0 text-body text-[0.85rem] text-left px-[0.6rem] py-[0.45rem] rounded-sm cursor-pointer w-full hover:bg-button-bg hover:text-red"
					onclick={() => {
						hide()
						store.closePack()
					}}
				>
					<X size={15} /><span>Close pack</span>
				</button>
			</div>
		{/snippet}
	</Dropdown>

	<nav class="flex items-center gap-[0.15rem]">
		<button
			class="inline-flex items-center gap-[0.35rem] bg-transparent border-0 text-[0.82rem] px-[0.7rem] py-[0.4rem] rounded-md cursor-pointer whitespace-nowrap hover:bg-button-bg hover:text-contrast {store.view ===
			'content'
				? 'bg-button-bg text-contrast font-semibold'
				: 'text-secondary font-medium'}"
			onclick={() => store.setView('content')}
		>
			<Layers size={15} /> Content
		</button>
		<button
			class="inline-flex items-center gap-[0.35rem] bg-transparent border-0 text-[0.82rem] px-[0.7rem] py-[0.4rem] rounded-md cursor-pointer whitespace-nowrap hover:bg-button-bg hover:text-contrast {store.view ===
			'files'
				? 'bg-button-bg text-contrast font-semibold'
				: 'text-secondary font-medium'}"
			onclick={() => store.setView('files')}
		>
			<Folder size={15} /> Files
		</button>
		<button
			class="inline-flex items-center gap-[0.35rem] bg-transparent border-0 text-[0.82rem] px-[0.7rem] py-[0.4rem] rounded-md cursor-pointer whitespace-nowrap hover:bg-button-bg hover:text-contrast {store.view ===
			'instance'
				? 'bg-button-bg text-contrast font-semibold'
				: 'text-secondary font-medium'}"
			onclick={() => store.setView('instance')}
		>
			<Plug size={15} /> Instance
			{#if syncCount}
				<span
					class="bg-brand text-on-brand rounded-max text-[0.62rem] font-semibold px-[0.4rem] py-[0.05rem] min-w-[1.1rem] text-center"
					>{syncCount}</span
				>
			{/if}
		</button>
		{#if store.git?.isRepo}
			<button
				class="inline-flex items-center gap-[0.35rem] bg-transparent border-0 text-[0.82rem] px-[0.7rem] py-[0.4rem] rounded-md cursor-pointer whitespace-nowrap hover:bg-button-bg hover:text-contrast {store.view ===
				'source'
					? 'bg-button-bg text-contrast font-semibold'
					: 'text-secondary font-medium'}"
				onclick={() => store.setView('source')}
			>
				<GitBranch size={15} /> Source
				{#if gitCount}
					<span
						class="bg-brand text-on-brand rounded-max text-[0.62rem] font-semibold px-[0.4rem] py-[0.05rem] min-w-[1.1rem] text-center"
						>{gitCount}</span
					>
				{/if}
			</button>
		{/if}
	</nav>

	<Dropdown placement="bottom-start">
		{#snippet trigger()}
			<button
				class="inline-flex items-center gap-[0.3rem] bg-bg-inset border border-divider text-secondary px-[0.5rem] py-[0.3rem] rounded-md text-[0.74rem] cursor-pointer ml-[0.2rem] whitespace-nowrap hover:border-divider-dark"
				use:tooltip={'Minecraft version & loader'}
			>
				{manifest?.minecraft} · <b class="text-contrast font-semibold"
					>{loaderLabel(manifest?.loader ?? '')}</b
				>
				<ChevronDown size={12} class="text-secondary shrink-0" />
			</button>
		{/snippet}
		{#snippet content(hide)}
			<PackSettings ondone={hide} />
		{/snippet}
	</Dropdown>

	<div class="flex-1 self-stretch min-w-[0.5rem]" data-tauri-drag-region></div>

	<Dropdown placement="bottom-end">
		{#snippet trigger()}
			<button
				class="inline-flex items-center gap-[0.4rem] text-[0.76rem] font-medium px-[0.6rem] py-[0.35rem] border-0 rounded-max bg-button-bg whitespace-nowrap cursor-pointer hover:bg-button-bg-hover {healthCls[
					health.kind
				]}"
				use:tooltip={'Pack health'}
			>
				<span class="w-[7px] h-[7px] rounded-full {dotCls[health.kind]}"></span>{health.label}
			</button>
		{/snippet}
		{#snippet content(_hide)}
			<div class="w-[320px] flex flex-col gap-px">
				{#if !problems.length}
					<div class="flex items-center gap-[0.4rem] text-green text-[0.82rem] px-[0.5rem] py-[0.4rem]">
						<CircleCheck size={15} /> No problems
					</div>
				{:else}
					<div
						class="text-[0.66rem] uppercase tracking-[0.05em] text-secondary font-[650] pt-[0.3rem] px-[0.5rem] pb-[0.35rem]"
					>
						{problems.length} to review
					</div>
					{#each problems as p (p.id)}
						<div
							class="flex items-start gap-2 px-[0.5rem] py-[0.4rem] rounded-sm border-l-2 hover:bg-button-bg {p.severity ===
							'error'
								? 'border-l-red'
								: 'border-l-orange'}"
						>
							<button
								class="flex-1 min-w-0 flex flex-col gap-[0.1rem] bg-transparent border-0 text-left cursor-pointer p-0"
								onclick={() => goToProblem(p)}
							>
								<span class="text-[0.82rem] font-semibold text-contrast">{p.title}</span>
								<span class="text-[0.72rem] text-secondary leading-[1.4]">{p.detail}</span>
							</button>
							{#if p.fix && p.fix.kind !== 'none'}
								<ButtonStyled
									size="small"
									type="outlined"
									onclick={() => p.fix && store.applyFix(p.fix)}
								>
									{p.fix.label}
								</ButtonStyled>
							{/if}
						</div>
					{/each}
				{/if}
			</div>
		{/snippet}
	</Dropdown>
	<ButtonStyled size="small" disabled={store.busy || !store.hasLock} onclick={() => onexport?.()}>
		<Package size={15} /> Export
	</ButtonStyled>
	<ButtonStyled size="small" color="brand" disabled={store.busy} onclick={() => onadd?.()}>
		<Plus size={15} /> Add
	</ButtonStyled>

	{#if !isMac}
		<WindowControls />
	{/if}

	{#if busy}
		<div class="absolute left-0 right-0 -bottom-px h-0.5 overflow-hidden">
			<span
				class="absolute h-full w-[35%] bg-brand rounded-max [animation:toolbar-loadbar-slide_1.1s_ease-in-out_infinite]"
			></span>
		</div>
	{/if}
</header>

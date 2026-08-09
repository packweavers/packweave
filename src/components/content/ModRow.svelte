<script lang="ts">
	import {
		ArrowLeftRight,
		ArrowUp,
		Check,
		ChevronDown,
		ExternalLink,
		Eye,
		EyeOff,
		Plus,
		RefreshCw,
		Trash2,
		X,
	} from '@lucide/svelte'
	import Avatar from '../ui/Avatar.svelte'
	import Badge from '../ui/Badge.svelte'
	import Dropdown from '../ui/Dropdown.svelte'
	import { tooltip } from '../../lib/tooltip'
	import { contextMenu } from '../../lib/contextmenu.svelte'
	import { store } from '../../lib/store.svelte'
	import { activeSource } from '../../types'
	import type { LockedMod, Validation } from '../../types'
	import {
		addableProviders,
		hasAlts,
		installedType,
		onVersionOpen,
		openPage,
		prerelease,
		providerLabel,
		providersOf,
		refreshVersions,
		setVersion,
		sideLabel,
		updateFor,
	} from '../../lib/mods'
	import { fromNow } from '../../util'

	function invalidVersion(m: LockedMod): Validation | null {
		return (
			store.validations.find(
				(v) => v.projectId === m.projectId && v.fix?.kind === 'update',
			) ?? null
		)
	}

	let {
		mod,
		index,
		selected,
		onselect,
		onkeyrow,
		onaddalt,
	}: {
		mod: LockedMod
		index: number
		selected: boolean
		onselect: (e: MouseEvent, index: number) => void
		onkeyrow: (e: KeyboardEvent, id: string) => void
		onaddalt: (mod: LockedMod, target: string) => void
	} = $props()

	let confirmRemove = $state(false)
	const meta = $derived(store.meta[mod.projectId])
	let refreshingVersions = $state(false)
</script>

<li
	role="button"
	tabindex={0}
	class={`group flex items-center gap-3 px-2.5 py-2 rounded-md border border-transparent cursor-pointer hover:bg-bg-raised hover:border-divider ${mod.disabled ? 'opacity-45' : ''} ${selected ? '!bg-button-bg !border-divider' : ''}`}
	onclick={(e) => onselect(e, index)}
	onkeydown={(e) => onkeyrow(e, mod.projectId)}
	use:contextMenu={() => [
		updateFor(mod) && {
			label: prerelease(updateFor(mod))
				? `Update to ${updateFor(mod)!.versionNumber} (${prerelease(updateFor(mod))})`
				: `Update to ${updateFor(mod)!.versionNumber}`,
			icon: ArrowUp,
			disabled: store.busy,
			onSelect: () => setVersion(mod, updateFor(mod)!.id),
		},
		{ separator: true },
		...providersOf(mod)
			.filter((p) => !p.preferred)
			.map((p) => ({
				label: `Prefer ${providerLabel(p.provider)}`,
				icon: Check,
				disabled: store.busy,
				onSelect: () => store.setPreferredSource(mod.projectId, p.provider),
			})),
		...addableProviders(mod).map((t) => ({
			label: `Add ${providerLabel(t)}`,
			icon: Plus,
			disabled: store.busy,
			onSelect: () => onaddalt(mod, t),
		})),
		...providersOf(mod)
			.filter((p) => !p.preferred)
			.map((p) => ({
				label: `Remove ${providerLabel(p.provider)} source`,
				icon: X,
				disabled: store.busy,
				onSelect: () => store.removeAltSource(mod.projectId, p.provider),
			})),
		(mod.preferred === 'modrinth' || mod.preferred === 'curseforge') && {
			label: `Open on ${providerLabel(mod.preferred)}`,
			icon: ExternalLink,
			onSelect: () => openPage(mod),
		},
		{ separator: true },
		{
			label: mod.disabled ? 'Enable' : 'Disable',
			icon: mod.disabled ? Eye : EyeOff,
			disabled: store.busy,
			onSelect: () => store.setDisabled(mod.projectId, !mod.disabled),
		},
		{
			label: 'Remove from pack',
			icon: Trash2,
			danger: true,
			disabled: store.busy,
			onSelect: () => store.requestDelete([mod.projectId]),
		},
	]}
>
	<Avatar src={meta?.iconUrl ?? null} alt={mod.name} size={38} />
	<div class="flex-1 min-w-0">
		<div class="flex items-center gap-2">
			<span class="font-semibold text-contrast whitespace-nowrap overflow-hidden text-ellipsis"
				>{mod.name}</span
			>
			{#if mod.disabled}<Badge>disabled</Badge>{/if}
		</div>
		<div class="flex items-center gap-md mt-1 text-tiny text-secondary">
			{#if meta?.author}
				<span class="inline-flex items-center gap-sm text-body">
					{#if meta?.authorIconUrl}
						<Avatar src={meta?.authorIconUrl ?? null} alt={meta?.author ?? ''} size={15} />
					{/if}
					{meta?.author}
				</span>
			{:else if !meta}
				<span class="inline-flex items-center gap-sm text-secondary">loading…</span>
			{/if}
			<span class="opacity-50">·</span>
			<span class="text-secondary">{sideLabel(mod)}</span>
			{#if !activeSource(mod)?.downloadUrl}
				<span class="text-orange">· no file</span>
			{/if}
		</div>
	</div>

	{#if installedType(mod)}
		<span
			class={`shrink-0 text-micro font-bold uppercase tracking-[0.03em] px-md py-2xs rounded-max ${
				installedType(mod) === 'beta' ? 'text-orange bg-orange/15' : 'text-red bg-red/15'
			}`}
			use:tooltip={'You are on a pre-release build'}
		>
			{installedType(mod)}
		</span>
	{/if}

	{#if invalidVersion(mod)}
		<button
			class="inline-flex items-center gap-1 text-white text-tiny font-semibold px-lg py-1 rounded-max cursor-pointer shrink-0 disabled:opacity-60 bg-orange"
			disabled={store.busy || !invalidVersion(mod)?.fix?.version}
			onclick={() => store.applyFix(invalidVersion(mod)!.fix!)}
			use:tooltip={'This version no longer exists. ' + (invalidVersion(mod)?.fix?.label ?? 'Update')}
		>
			<ArrowUp size={13} /> Update
		</button>
	{:else if updateFor(mod)}
		<button
			class={`inline-flex items-center gap-1 text-on-brand text-tiny font-semibold px-lg py-1 rounded-max cursor-pointer shrink-0 disabled:opacity-60 ${
				prerelease(updateFor(mod)) ? 'bg-orange text-white' : 'bg-brand hover:bg-brand-hover'
			}`}
			disabled={store.busy}
			onclick={() => setVersion(mod, updateFor(mod)!.id)}
			use:tooltip={prerelease(updateFor(mod))
				? `Update to ${updateFor(mod)!.versionNumber} (${prerelease(updateFor(mod))} pre-release)`
				: `Update to ${updateFor(mod)!.versionNumber}`}
		>
			<ArrowUp size={13} /> Update
		</button>
	{/if}

	<Dropdown placement="bottom-end">
		{#snippet trigger()}
			<button
				class="inline-flex items-center gap-1 max-w-[12rem] bg-bg-inset border border-divider text-body text-tiny px-lg py-1 rounded-sm cursor-pointer shrink-0 font-mono hover:border-divider-dark hover:text-contrast"
				use:tooltip={activeSource(mod)?.versionNumber ?? ''}
				onclick={() => onVersionOpen(mod)}
			>
				<span class="overflow-hidden text-ellipsis whitespace-nowrap min-w-0"
					>{activeSource(mod)?.versionNumber}</span
				>
				<ChevronDown size={12} class="shrink-0" />
			</button>
		{/snippet}
		{#snippet content(hide)}
			<div class="min-w-[230px] max-w-[340px] max-h-[320px] overflow-y-auto flex flex-col gap-px">
				<button
					class="flex items-center gap-2 bg-transparent border-none text-body text-small text-left px-lg py-md rounded-sm cursor-pointer w-full hover:bg-button-bg hover:text-contrast disabled:opacity-50"
					disabled={refreshingVersions}
					onclick={async () => {
						refreshingVersions = true
						try {
							await refreshVersions(mod)
						} finally {
							refreshingVersions = false
						}
					}}
				>
					<RefreshCw size={13} class={refreshingVersions ? 'animate-spin' : ''} />
					<span>{refreshingVersions ? 'Checking…' : 'Check for new versions'}</span>
				</button>
				<div class="h-px bg-divider my-1"></div>
				{#if !store.versions[mod.projectId]}
					<div class="p-2.5 text-secondary text-small text-center">Loading versions…</div>
				{/if}
				{#each store.versions[mod.projectId] ?? [] as v (v.id)}
					<button
						class={`flex items-center gap-2 bg-transparent border-none text-small text-left px-lg py-md rounded-sm cursor-pointer w-full hover:bg-button-bg hover:text-contrast ${
							v.id === activeSource(mod)?.versionId ? 'text-contrast' : 'text-body'
						}`}
						onclick={() => {
							hide()
							setVersion(mod, v.id)
						}}
					>
						<span class="font-mono flex-1 min-w-0 whitespace-nowrap overflow-hidden text-ellipsis"
							>{v.versionNumber}</span
						>
						<span
							class={`shrink-0 text-micro uppercase tracking-[0.03em] ${
								v.versionType === 'release'
									? 'text-green'
									: v.versionType === 'beta'
										? 'text-orange'
										: v.versionType === 'alpha'
											? 'text-red'
											: 'text-secondary'
							}`}>{v.versionType}</span
						>
						<span class="shrink-0 text-micro text-secondary">{fromNow(v.datePublished)}</span>
						{#if v.id === activeSource(mod)?.versionId}
							<Check size={13} class="text-brand" />
						{/if}
					</button>
				{/each}
			</div>
		{/snippet}
	</Dropdown>

	<Dropdown placement="bottom-end">
		{#snippet trigger()}
			<button
				class={`grid place-items-center w-[1.8rem] h-[1.8rem] rounded-sm border-none bg-transparent cursor-pointer shrink-0 hover:bg-button-bg hover:text-contrast ${
					hasAlts(mod) ? 'opacity-100 text-brand' : 'opacity-0 group-hover:opacity-100 text-secondary'
				}`}
				aria-label="Manage providers"
				use:tooltip={hasAlts(mod) ? 'Multiple providers' : 'Manage providers'}
			>
				<ArrowLeftRight size={14} />
			</button>
		{/snippet}
		{#snippet content(hide)}
			<div class="min-w-[220px] max-w-[340px] max-h-[320px] overflow-y-auto flex flex-col gap-px">
				<div
					class="text-micro uppercase tracking-[0.05em] text-secondary font-[650] px-lg pt-sm pb-1"
				>
					Providers
				</div>
				{#each providersOf(mod) as p (p.provider)}
					<div class="flex items-center gap-md px-lg py-sm">
						<span class="flex-1 flex items-center gap-md text-small text-contrast min-w-0">
							{providerLabel(p.provider)}
							{#if !p.resolved}
								<span
									class="text-micro uppercase tracking-[0.03em] text-orange"
									use:tooltip={'No compatible version on this provider'}>unavailable</span
								>
							{/if}
						</span>
						{#if p.preferred}
							<span class="text-micro uppercase tracking-[0.04em] font-[650] text-brand"
								>preferred</span
							>
						{:else}
							<button
								class="bg-button-bg border-none text-body text-tiny font-medium px-2 py-2xs rounded-sm cursor-pointer hover:bg-button-bg-hover hover:text-contrast disabled:opacity-60"
								disabled={store.busy}
								onclick={() => {
									hide()
									store.setPreferredSource(mod.projectId, p.provider)
								}}
							>
								Prefer
							</button>
							<button
								class="grid place-items-center w-[1.35rem] h-[1.35rem] rounded-sm border-none bg-transparent text-secondary cursor-pointer hover:bg-button-bg hover:text-red disabled:opacity-60"
								aria-label="Remove this source"
								use:tooltip={'Remove this source'}
								disabled={store.busy}
								onclick={() => store.removeAltSource(mod.projectId, p.provider)}
							>
								<X size={12} />
							</button>
						{/if}
					</div>
				{/each}
				{#if addableProviders(mod).length}
					<div class="h-px bg-divider my-1"></div>
					{#each addableProviders(mod) as t (t)}
						<button
							class="flex items-center gap-2 bg-transparent border-none text-body text-small text-left px-lg py-md rounded-sm cursor-pointer w-full hover:bg-button-bg hover:text-contrast disabled:opacity-60"
							disabled={store.busy}
							onclick={() => {
								hide()
								onaddalt(mod, t)
							}}
						>
							<Plus size={13} /> Add {providerLabel(t)}
						</button>
					{/each}
				{/if}
			</div>
		{/snippet}
	</Dropdown>

	{#if mod.preferred === 'modrinth' || mod.preferred === 'curseforge'}
		<button
			class="grid place-items-center w-[1.8rem] h-[1.8rem] rounded-sm border-none bg-transparent text-secondary cursor-pointer opacity-0 group-hover:opacity-100 shrink-0 hover:bg-button-bg hover:text-contrast"
			aria-label={`Open on ${providerLabel(mod.preferred)}`}
			use:tooltip={`Open on ${providerLabel(mod.preferred)}`}
			onclick={() => openPage(mod)}
		>
			<ExternalLink size={14} />
		</button>
	{/if}
	{#if confirmRemove}
		<span class="inline-flex items-center gap-1" role="presentation" onclick={(e) => e.stopPropagation()}>
			<button
				class="bg-red text-white text-tiny font-semibold px-2 py-1 rounded-sm cursor-pointer disabled:opacity-60"
				disabled={store.busy}
				onclick={() => {
					confirmRemove = false
					store.requestDelete([mod.projectId])
				}}
			>
				Remove
			</button>
			<button
				class="bg-button-bg text-body text-tiny px-2 py-1 rounded-sm cursor-pointer hover:text-contrast"
				onclick={() => (confirmRemove = false)}>No</button
			>
		</span>
	{:else}
		<button
			class="grid place-items-center w-[1.8rem] h-[1.8rem] rounded-sm border-none bg-transparent text-secondary cursor-pointer opacity-0 group-hover:opacity-100 shrink-0 hover:bg-button-bg hover:text-red disabled:opacity-60"
			aria-label="Remove"
			use:tooltip={'Remove'}
			disabled={store.busy}
			onclick={() => (confirmRemove = true)}
		>
			<Trash2 size={14} />
		</button>
	{/if}
</li>

<script lang="ts">
	import { onMount } from 'svelte'
	import {
		RefreshCw,
		Unlink,
		FolderOpen,
		CircleCheck,
		ChevronRight,
		ArrowDownToLine,
		ArrowUpFromLine,
		EyeOff,
		Link2,
		Plug,
	} from '@lucide/svelte'
	import ButtonStyled from './ui/ButtonStyled.svelte'
	import FeatureTip from './ui/FeatureTip.svelte'
	import { store } from '../lib/store.svelte'
	import { contextMenu } from '../lib/contextmenu.svelte'
	import { tooltip } from '../lib/tooltip'
	import { api, revealFolder } from '../api'
	import type { EnvChange, FileChange, ModChange, SyncDirection, SyncOp } from '../types'
	import { basename, loaderLabel } from '../util'

	let { onlink }: { onlink?: () => void } = $props()

	interface Action {
		dir: SyncDirection
		label: string
	}

	onMount(() => {
		if (store.instanceDir) store.refreshSync(true)
	})

	const choices = $state<Record<string, SyncDirection>>({})
	const expanded = $state<Record<string, boolean>>({})
	const diffs = $state<Record<string, string>>({})
	const diffLoading = $state<Record<string, boolean>>({})

	async function toggleDiff(f: FileChange) {
		const id = fileId(f)
		if (expanded[id]) {
			expanded[id] = false
			return
		}
		expanded[id] = true
		if (diffs[id] === undefined && store.pack) {
			diffLoading[id] = true
			try {
				diffs[id] = await api.syncFileDiff(store.pack.dir, f.path, f.kind)
			} catch (e) {
				diffs[id] = `error: ${e}`
			} finally {
				diffLoading[id] = false
			}
		}
	}

	function diffLines(id: string) {
		return (diffs[id] ?? '').split('\n').map((text) => {
			const c = text[0]
			const cls = c === '+' ? 'add' : c === '-' ? 'del' : c === '⋯' ? 'skip' : 'ctx'
			return { text, cls }
		})
	}

	const modChanges = $derived(store.sync?.mods ?? [])
	const fileChanges = $derived(store.sync?.files ?? [])
	const envChange = $derived<EnvChange | null>(store.sync?.env ?? null)

	function envSide(mc: string | null, loader: string | null, ver: string | null): string {
		const m = mc || '?'
		if (!loader || loader === 'vanilla') return `${m} · Vanilla`
		return `${m} · ${loaderLabel(loader)}${ver ? ` ${ver}` : ''}`
	}

	const envActions = $derived.by<Action[]>(() => {
		const e = envChange
		if (!e) return []
		const acts: Action[] = [{ dir: 'pull', label: 'Use instance' }]
		if (e.writable) acts.push({ dir: 'push', label: 'Use pack' })
		return acts
	})

	const modId = (m: ModChange) => `m:${m.kind}:${m.projectId ?? m.relPath ?? m.filename ?? m.name}`
	const fileId = (f: FileChange) => `f:${f.kind}:${f.path}`

	function modActions(m: ModChange): Action[] {
		switch (m.kind) {
			case 'instance_only':
				return [
					{ dir: 'pull', label: 'Add to pack' },
					{ dir: 'push', label: 'Remove from instance' },
					{ dir: 'ignore', label: 'Ignore' },
				]
			case 'pack_only':
				return m.dependency
					? [{ dir: 'push', label: 'Install' }]
					: [
							{ dir: 'pull', label: 'Remove from pack' },
							{ dir: 'push', label: 'Install' },
						]
			case 'version_diff':
				return [
					{ dir: 'pull', label: 'Use instance version' },
					{ dir: 'push', label: 'Use pack version' },
				]
			case 'unknown':
				return [
					{ dir: 'pull', label: 'Add as override' },
					{ dir: 'push', label: 'Remove from instance' },
					{ dir: 'ignore', label: 'Ignore' },
				]
			case 'local_changed':
				return [
					{ dir: 'pull', label: 'Update pack' },
					{ dir: 'push', label: 'Revert instance' },
				]
			case 'local_only':
				return [
					{ dir: 'push', label: 'Install' },
					{ dir: 'pull', label: 'Remove from pack' },
				]
			case 'disabled':
				return [
					{ dir: 'push', label: 'Remove from instance' },
					{ dir: 'pull', label: 'Enable in pack' },
				]
		}
	}

	function fileActions(f: FileChange): Action[] {
		switch (f.kind) {
			case 'new':
				return [
					{ dir: 'pull', label: 'Add to pack' },
					{ dir: 'ignore', label: 'Ignore' },
				]
			case 'changed':
				return [
					{ dir: 'pull', label: 'Update pack' },
					{ dir: 'push', label: 'Revert instance' },
				]
			case 'removed':
				return [
					{ dir: 'pull', label: 'Remove from pack' },
					{ dir: 'push', label: 'Restore' },
				]
		}
	}

	function modDesc(m: ModChange): string {
		switch (m.kind) {
			case 'instance_only':
				return `In instance, not in pack${m.instanceVersion ? ` · ${m.instanceVersion}` : ''}`
			case 'pack_only':
				return `In pack, missing from instance${m.packVersion ? ` · ${m.packVersion}` : ''}`
			case 'version_diff':
				return `Instance ${m.instanceVersion} → pack ${m.packVersion}`
			case 'unknown':
				return `Unrecognized file · ${m.relPath ?? m.filename}`
			case 'local_changed':
				return `Pack copy differs from the instance · ${m.relPath}`
			case 'local_only':
				return `Bundled in the pack, not installed · ${m.relPath}`
			case 'disabled':
				return `Disabled in pack, still in instance${m.instanceVersion ? ` · ${m.instanceVersion}` : ''}`
		}
	}

	function fileDesc(f: FileChange): string {
		switch (f.kind) {
			case 'new':
				return 'New file in your instance'
			case 'changed':
				return 'Edited in your instance'
			case 'removed':
				return 'In the pack, deleted from your instance'
		}
	}

	function setChoice(id: string, dir: SyncDirection) {
		if (choices[id] === dir) delete choices[id]
		else choices[id] = dir
	}

	const selectedCount = $derived(Object.keys(choices).length)

	function pullAll() {
		if (envChange) choices['env'] = 'pull'
		for (const m of modChanges) {
			if (modActions(m).some((a) => a.dir === 'pull')) choices[modId(m)] = 'pull'
		}
		for (const f of fileChanges) {
			if (fileActions(f).some((a) => a.dir === 'pull')) choices[fileId(f)] = 'pull'
		}
	}

	function pushAll() {
		if (envChange?.writable) choices['env'] = 'push'
		for (const m of modChanges) {
			if (modActions(m).some((a) => a.dir === 'push')) choices[modId(m)] = 'push'
		}
		for (const f of fileChanges) {
			if (fileActions(f).some((a) => a.dir === 'push')) choices[fileId(f)] = 'push'
		}
	}

	function clearChoices() {
		for (const k of Object.keys(choices)) delete choices[k]
	}

	async function apply() {
		const ops: SyncOp[] = []
		if (envChange && choices['env']) {
			ops.push({ target: 'env', kind: 'env', direction: choices['env'] })
		}
		for (const m of modChanges) {
			const d = choices[modId(m)]
			if (d)
				ops.push({
					target: 'mod',
					kind: m.kind,
					direction: d,
					projectId: m.projectId,
					slug: m.slug,
					provider: m.provider,
					name: m.name,
					instanceVersionId: m.instanceVersionId,
					filename: m.filename,
					relPath: m.relPath,
					projectType: m.projectType,
				})
		}
		for (const f of fileChanges) {
			const d = choices[fileId(f)]
			if (d) ops.push({ target: 'file', kind: f.kind, direction: d, path: f.path })
		}
		clearChoices()
		await store.applySyncOps(ops)
	}

	function actIcon(dir: SyncDirection) {
		if (dir === 'pull') return ArrowDownToLine
		if (dir === 'push') return ArrowUpFromLine
		return EyeOff
	}

	function actClass(dir: SyncDirection, on: boolean): string {
		const base =
			'text-xs font-medium px-2 py-1 rounded-sm border whitespace-nowrap cursor-pointer'
		if (!on)
			return `${base} border-divider bg-button-bg text-secondary hover:text-contrast`
		if (dir === 'pull') return `${base} bg-brand-highlight border-green text-green`
		if (dir === 'push') return `${base} bg-blue/15 border-blue text-blue`
		return `${base} bg-surface-4 border-divider-dark text-body`
	}

	function dlineClass(cls: string): string {
		if (cls === 'add') return 'whitespace-pre px-3 bg-green/15 text-green'
		if (cls === 'del') return 'whitespace-pre px-3 bg-red/15 text-red'
		if (cls === 'skip') return 'whitespace-pre px-3 text-divider-dark text-center'
		return 'whitespace-pre px-3 text-secondary'
	}
</script>

<div class="h-full overflow-y-auto px-5 pt-[1.1rem] pb-8 max-w-[920px] mx-auto w-full">
	{#if !store.bound}
		<div class="h-full flex flex-col items-center justify-center text-center gap-4 py-16">
			<div class="grid place-items-center w-14 h-14 rounded-max bg-button-bg text-secondary">
				<Plug size={26} />
			</div>
			<div class="max-w-[360px]">
				<div class="text-[1.05rem] font-semibold text-contrast mb-1">No instance linked</div>
				<div class="text-[0.85rem] text-secondary leading-relaxed">
					Link a Minecraft instance to sync this pack's files to and from
					your launcher.
				</div>
			</div>
			<ButtonStyled color="brand" onclick={() => onlink?.()}>
				<Link2 size={15} />
				Link an instance…
			</ButtonStyled>
		</div>
	{:else}
	<FeatureTip id="instance" class="mb-3">
		This is your live test setup. Play the linked instance, then pull the file changes you made
		in-game back into the pack; or push pack changes out to it.
	</FeatureTip>
	<div class="flex items-center justify-between gap-2 pb-3 mb-2 border-b border-divider">
		<button
			class="inline-flex items-center gap-1 bg-transparent border-0 text-secondary cursor-pointer text-[0.78rem] hover:text-body"
			use:tooltip={store.instanceDir ?? ''}
			aria-label="Reveal instance folder"
			onclick={() => store.instanceDir && revealFolder(store.instanceDir)}
		>
			<FolderOpen size={13} />
			{store.instanceDir ? basename(store.instanceDir) : 'no instance'}
		</button>
		<div class="flex gap-1">
			<ButtonStyled size="small" type="transparent" disabled={store.scanning} onclick={() => store.refreshSync()}>
				<RefreshCw size={14} class={store.scanning ? 'animate-spin' : ''} />
				Rescan
			</ButtonStyled>
			<ButtonStyled size="small" type="transparent" color="red" onclick={() => store.unlinkInstance()}>
				<Unlink size={14} />
				Unlink
			</ButtonStyled>
		</div>
	</div>

	<div class="flex items-start gap-2.5 pt-3 px-1.5 pb-1">
		<button
			class={`flex-shrink-0 w-[34px] h-5 rounded-max border p-0.5 cursor-pointer mt-0.5 transition-colors ${store.autoPushOnSave ? 'bg-brand border-brand' : 'bg-button-bg border-divider'}`}
			role="switch"
			aria-checked={store.autoPushOnSave}
			aria-label="Automatically push edits on save"
			onclick={() => store.setAutoPush(!store.autoPushOnSave)}
		>
			<span
				class={`block w-3.5 h-3.5 rounded-full transition-transform ${store.autoPushOnSave ? 'translate-x-3.5 bg-on-brand' : 'bg-secondary'}`}
			></span>
		</button>
		<div class="min-w-0">
			<div class="text-[0.82rem] font-semibold text-contrast">Automatically push edits on save</div>
		</div>
	</div>

	{#if store.scanning && !store.sync}
		<div class="text-center text-secondary p-8">Scanning instance…</div>
	{:else if store.sync?.inSync}
		<div class="text-center text-green py-8 px-4 flex flex-col items-center gap-2">
			<CircleCheck size={30} />
			<div>Pack and instance are in sync.</div>
		</div>
	{:else if store.sync}
		<div class="-mx-1 px-1">
			{#if envChange}
				<div class="text-[0.68rem] uppercase tracking-[0.06em] text-secondary font-bold pt-3 px-0.5 pb-1.5">
					Minecraft & loader
				</div>
				<div
					class="flex items-center justify-between gap-2.5 py-2 px-1.5 rounded-sm hover:bg-bg-raised"
					use:contextMenu={() =>
						envActions.map((a) => ({
							label: a.label,
							icon: actIcon(a.dir),
							onSelect: () => setChoice('env', a.dir),
						}))}
				>
					<div class="min-w-0">
						<div class="font-semibold text-contrast text-[0.84rem] whitespace-nowrap overflow-hidden text-ellipsis">
							{envSide(envChange.packMinecraft, envChange.packLoader, envChange.packLoaderVersion)}
						</div>
						<div class="text-[0.72rem] text-secondary">
							Instance has {envSide(envChange.instanceMinecraft, envChange.instanceLoader, envChange.instanceLoaderVersion)}
						</div>
					</div>
					<div class="flex gap-1 flex-shrink-0">
						{#each envActions as a (a.dir)}
							<button class={actClass(a.dir, choices['env'] === a.dir)} onclick={() => setChoice('env', a.dir)}>
								{a.label}
							</button>
						{/each}
					</div>
				</div>
			{/if}

			{#if modChanges.length}
				<div class="text-[0.68rem] uppercase tracking-[0.06em] text-secondary font-bold pt-3 px-0.5 pb-1.5">Mods</div>
			{/if}
			{#each modChanges as m (modId(m))}
				<div
					class="flex items-center justify-between gap-2.5 py-2 px-1.5 rounded-sm hover:bg-bg-raised"
					use:contextMenu={() =>
						modActions(m).map((a) => ({
							label: a.label,
							icon: actIcon(a.dir),
							onSelect: () => setChoice(modId(m), a.dir),
						}))}
				>
					<div class="min-w-0">
						<div class="font-semibold text-contrast text-[0.84rem] whitespace-nowrap overflow-hidden text-ellipsis">
							{m.name}
						</div>
						<div class="text-[0.72rem] text-secondary">{modDesc(m)}</div>
					</div>
					<div class="flex gap-1 flex-shrink-0">
						{#each modActions(m) as a (a.dir)}
							<button class={actClass(a.dir, choices[modId(m)] === a.dir)} onclick={() => setChoice(modId(m), a.dir)}>
								{a.label}
							</button>
						{/each}
					</div>
				</div>
			{/each}

			{#if fileChanges.length}
				<div class="text-[0.68rem] uppercase tracking-[0.06em] text-secondary font-bold pt-3 px-0.5 pb-1.5">Files</div>
			{/if}
			{#each fileChanges as f (fileId(f))}
				<div class="rounded-sm">
					<div
						class="flex items-center justify-between gap-2.5 py-2 px-1.5 rounded-sm hover:bg-bg-raised"
						use:contextMenu={() =>
							fileActions(f).map((a) => ({
								label: a.label,
								icon: actIcon(a.dir),
								onSelect: () => setChoice(fileId(f), a.dir),
							}))}
					>
						<button
							class="bg-transparent border-0 text-secondary cursor-pointer grid place-items-center p-0.5 flex-shrink-0"
							use:tooltip={'Toggle diff'}
							aria-label="Toggle diff"
							onclick={() => toggleDiff(f)}
						>
							<ChevronRight size={14} class={`transition-transform ${expanded[fileId(f)] ? 'rotate-90' : ''}`} />
						</button>
						<div class="min-w-0 cursor-pointer" onclick={() => toggleDiff(f)}>
							<div class="font-semibold text-contrast text-[0.84rem] whitespace-nowrap overflow-hidden text-ellipsis">
								{f.path}
							</div>
							<div class="text-[0.72rem] text-secondary">{fileDesc(f)}</div>
						</div>
						<div class="flex gap-1 flex-shrink-0">
							{#each fileActions(f) as a (a.dir)}
								<button class={actClass(a.dir, choices[fileId(f)] === a.dir)} onclick={() => setChoice(fileId(f), a.dir)}>
									{a.label}
								</button>
							{/each}
						</div>
					</div>
					{#if expanded[fileId(f)]}
						<div
							class="mt-0.5 mb-1.5 ml-[1.6rem] bg-bg border border-divider rounded-sm max-h-64 overflow-auto py-1.5 font-mono text-[0.74rem] leading-[1.45]"
						>
							{#if diffLoading[fileId(f)]}
								<div class="py-2 px-3 text-secondary">Loading diff…</div>
							{:else}
								{#each diffLines(fileId(f)) as line, i (i)}
									<div class={dlineClass(line.cls)}>{line.text || ' '}</div>
								{/each}
							{/if}
						</div>
					{/if}
				</div>
			{/each}
		</div>
	{/if}

	{#if store.sync && !store.sync.inSync}
		<div class="flex items-center justify-between mt-3 pt-3 border-t border-divider">
			<div class="flex gap-0.5">
				<ButtonStyled size="small" type="transparent" onclick={pullAll}>Pull all</ButtonStyled>
				<ButtonStyled size="small" type="transparent" onclick={pushAll}>Push all</ButtonStyled>
				{#if selectedCount}
					<ButtonStyled size="small" type="transparent" onclick={clearChoices}>Clear</ButtonStyled>
				{/if}
			</div>
			<ButtonStyled color="brand" disabled={store.busy || selectedCount === 0} onclick={apply}>
				Apply {selectedCount || ''}
			</ButtonStyled>
		</div>
	{/if}
	{/if}
</div>

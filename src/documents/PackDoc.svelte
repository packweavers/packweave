<script lang="ts">
	import {
		Asterisk,
		ChevronDown,
		ChevronRight,
		File,
		Folder,
		Lock,
		SlidersHorizontal,
	} from '@lucide/svelte'
	import { onMount, type Component } from 'svelte'

	import ConfirmDialog from '@/dialogs/ConfirmDialog.svelte'
	import { DATAPACK_FOLDERS } from '@/lib/content/types'
	import { pathHints } from '@/lib/files/paths'
	import { rulesUnder, ruleTree, type RuleKind, type RuleNode } from '@/lib/files/rules'
	import { IGNORE_FILES, openInFiles, openSetting, resetIgnoreFile } from '@/lib/flows/files'
	import { addRemote, initRepo, withGit } from '@/lib/flows/git'
	import { linkInstance } from '@/lib/flows/instance'
	import { chooseDatapackFolder } from '@/lib/flows/pack'
	import { app } from '@/lib/state/app.svelte'
	import { content } from '@/lib/state/content.svelte'
	import { dialogs } from '@/lib/state/dialogs.svelte'
	import { files } from '@/lib/state/files.svelte'
	import { git } from '@/lib/state/git.svelte'
	import { instance } from '@/lib/state/instance.svelte'
	import { preserve, PRESERVE_FILE } from '@/lib/state/preserve.svelte'
	import { tabs, type Doc } from '@/lib/state/tabs.svelte'
	import { tasks } from '@/lib/state/tasks.svelte'
	import { toasts } from '@/lib/state/toasts.svelte'
	import { ws } from '@/lib/state/workspace.svelte'
	import { loaderVersions, minecraftVersions } from '@/lib/tauri/catalog'
	import { removeRemote, setRemoteUrl } from '@/lib/tauri/git'
	import { readIgnore, writeIgnore } from '@/lib/tauri/instance'
	import { setPackIcon } from '@/lib/tauri/pack'
	import type { Channel, IgnoreKind, Manifest } from '@/lib/tauri/types'
	import { pickFile } from '@/lib/tauri/windows'
	import { loaderName, loaderVersionOptions } from '@/lib/util/format'
	import { instanceFolderName, shortenHome } from '@/lib/util/path'
	import CodeEditor from '@/ui/CodeEditor.svelte'
	import Icon from '@/ui/Icon.svelte'
	import PathField from '@/ui/PathField.svelte'
	import { pending } from '@/ui/pending'
	import Segmented from '@/ui/Segmented.svelte'
	import Select from '@/ui/Select.svelte'

	interface Props {
		doc: Doc
	}

	let { doc }: Props = $props()

	type Section = 'general' | 'game' | 'instances' | 'sources' | 'ignored' | 'edits' | 'git'

	const IGNORES = (['git', 'export'] as IgnoreKind[]).map((kind) => ({
		kind,
		...IGNORE_FILES[kind],
	}))

	const SECTIONS: { id: Section; label: string }[] = [
		{ id: 'general', label: 'General' },
		{ id: 'game', label: 'Game' },
		{ id: 'instances', label: 'Instances' },
		{ id: 'sources', label: 'Sources' },
		{ id: 'ignored', label: 'Ignored' },
		{ id: 'edits', label: 'Player edits' },
		{ id: 'git', label: 'Git and remote' },
	]

	let versions = $state<string[]>([])
	let targetVersion = $state(ws.manifest?.minecraft ?? '')
	let loaders = $state<string[]>([])
	let loaderChoice = $state('vanilla')
	let ignores = $state<Record<IgnoreKind, string>>({ git: '', export: '' })
	let savedIgnores = $state<Record<IgnoreKind, string>>({ git: '', export: '' })
	let newPattern = $state('')
	let collapsedEdits = $state<Record<string, boolean>>({})
	let packPaths = $state<string[]>([])
	let datapackDraft = $state('')
	let savingDatapacks = false

	const section = $derived(
		SECTIONS.some((s) => s.id === doc.arg) ? (doc.arg as Section) : 'general',
	)
	const manifest = $derived(ws.manifest)
	const minecraft = $derived(manifest?.minecraft ?? '')
	const loader = $derived(manifest?.loader ?? 'vanilla')
	const main = $derived(ws.mainDir ?? ws.dir ?? '')
	const packPrefs = $derived(app.pack(main))

	onMount(() => {
		void minecraftVersions(false)
			.then((v) => (versions = v))
			.catch((error) => toasts.error(error))
	})

	$effect(() => {
		if (!minecraft || loader === 'vanilla') return
		void loaderVersions(loader, minecraft)
			.then((found) => (loaders = found.versions.slice(0, 50)))
			.catch((error) => {
				loaders = []
				tasks.fail('Loading loader versions', error)
			})
	})

	$effect(() => {
		loaderChoice = loader
	})

	$effect(() => {
		datapackDraft = manifest?.datapacks ?? ''
	})

	$effect(() => {
		const dir = ws.dir
		if (!dir || section !== 'ignored') return
		void files.packPaths().then((found) => (packPaths = found))
		for (const { kind } of IGNORES) {
			void readIgnore(dir, kind)
				.then((text) => {
					ignores[kind] = text
					savedIgnores[kind] = text
				})
				.catch((error) => toasts.error(error))
		}
	})

	async function change(next: Partial<Manifest>, resolve: boolean): Promise<void> {
		if (!manifest) return
		try {
			await ws.saveManifest({ ...manifest, ...next })
			ws.changed()
			if (resolve) await content.resolve()
		} catch (error) {
			toasts.error(error)
		}
	}

	async function newestLoader(kind: string, mc: string): Promise<string | null> {
		try {
			const newest = (await loaderVersions(kind, mc)).versions[0] ?? null
			if (!newest) toasts.error(`${loaderName(kind)} has no version for Minecraft ${mc}`)
			return newest
		} catch (error) {
			toasts.error(error)
			return null
		}
	}

	async function switchMinecraft(next: string): Promise<void> {
		if (!manifest || next === manifest.minecraft) return
		if (manifest.loader === 'vanilla') return change({ minecraft: next }, true)
		const newest = await newestLoader(manifest.loader, next)
		if (newest) await change({ minecraft: next, loaderVersion: newest }, true)
	}

	async function switchLoader(next: string): Promise<void> {
		if (!manifest || next === manifest.loader) return
		if (next === 'vanilla') return change({ loader: next, loaderVersion: null }, true)
		const newest = await newestLoader(next, manifest.minecraft)
		if (newest) await change({ loader: next, loaderVersion: newest }, true)
		loaderChoice = ws.manifest?.loader ?? 'vanilla'
	}

	async function setDatapacks(): Promise<void> {
		const folder = datapackDraft.trim().replace(/\/+$/, '')
		if (savingDatapacks || !folder || folder === manifest?.datapacks) return
		savingDatapacks = true
		if (!(await chooseDatapackFolder(folder))) datapackDraft = manifest?.datapacks ?? ''
		savingDatapacks = false
	}

	async function datapackHints(text: string): Promise<string[]> {
		const presets = DATAPACK_FOLDERS.map((f) => `${f.folder}/`)
		const folders = (await files.gamePaths()).filter((p) => p.endsWith('/'))
		if (!text.trim()) return presets
		return pathHints([...new Set([...presets, ...folders])], text)
	}

	function ignoreHints(line: string): string[] {
		return pathHints(packPaths, line)
	}

	async function pickIcon(): Promise<void> {
		try {
			const file = await pickFile('Choose an icon', [{ name: 'Images', extensions: ['png'] }])
			if (!file || !ws.dir) return
			await setPackIcon(ws.dir, file)
			await ws.refreshIcon()
			ws.changed()
		} catch (error) {
			toasts.error(error)
		}
	}

	async function clearIcon(): Promise<void> {
		if (!ws.dir) return
		try {
			await setPackIcon(ws.dir, null)
			await ws.refreshIcon()
			ws.changed()
		} catch (error) {
			toasts.error(error)
		}
	}

	async function saveIgnore(kind: IgnoreKind): Promise<void> {
		if (!ws.dir) return
		await writeIgnore(ws.dir, kind, ignores[kind])
		savedIgnores[kind] = ignores[kind]
		ws.changed()
		if (kind === 'git') void instance.scan()
		toasts.show(`Saved ${IGNORE_FILES[kind].file}`)
	}

	async function resetIgnore(kind: IgnoreKind): Promise<void> {
		const dir = ws.dir
		if (!dir || !(await resetIgnoreFile(kind))) return
		const text = await readIgnore(dir, kind)
		ignores[kind] = text
		savedIgnores[kind] = text
		if (kind === 'git') void instance.scan()
	}

	const KIND_ICONS: Record<RuleKind, Component> = {
		folder: Folder,
		file: File,
		glob: Asterisk,
		setting: SlidersHorizontal,
	}

	const editTree = $derived(ruleTree(preserve.rules))

	function addPattern(): void {
		if (!newPattern.trim()) return
		void preserve.add(newPattern).then((ok) => {
			if (ok) newPattern = ''
		})
	}

	function openRule(item: RuleNode): void {
		if (item.kind === 'glob') return
		if (item.kind === 'setting' && item.key) {
			void openSetting(`overrides/${item.path}`, item.key)
			return
		}
		void openInFiles(`overrides/${item.path}`, item.kind === 'folder')
	}

	function onEditClick(event: MouseEvent, item: RuleNode): void {
		if ((event.target as Element).closest('.row-actions')) return
		if (item.children.length > 0) collapsedEdits[item.id] = !collapsedEdits[item.id]
	}

	function onEditDouble(event: MouseEvent, item: RuleNode): void {
		if ((event.target as Element).closest('.row-actions')) return
		openRule(item)
	}

	async function removeRules(item: RuleNode): Promise<void> {
		const lines = rulesUnder(item).map((r) => r.line)
		if (lines.length > 1) {
			const ok = await dialogs.open(ConfirmDialog, {
				title: `Remove ${lines.length} rules?`,
				message: `Everything under ${item.name}`,
				confirm: 'Remove',
				danger: true,
			})
			if (!ok) return
		}
		await preserve.remove(lines)
	}

	async function resetPreserve(): Promise<void> {
		const ok = await dialogs.open(ConfirmDialog, {
			title: 'Reset player edits to defaults?',
			message: 'Entries you added are removed.',
			confirm: 'Reset',
			danger: true,
		})
		if (ok) await preserve.reset()
	}

	async function removePack(): Promise<void> {
		const ok = await dialogs.open(ConfirmDialog, {
			title: `Remove ${ws.name} from packweave?`,
			message: "The folder isn't deleted.",
			confirm: 'Remove pack',
			danger: true,
		})
		if (!ok) return
		app.removeRecent(main)
		await ws.close()
	}

	function instanceName(dir: string | null): string {
		return dir ? instanceFolderName(dir) : 'none'
	}
</script>

{#snippet editNode(item: RuleNode, depth: number)}
	{@const rule = item.rules.at(-1)}
	<div
		class="trow"
		style:padding-left="{16 + depth * 16}px"
		role="treeitem"
		aria-selected="false"
		aria-expanded={item.children.length > 0 ? !collapsedEdits[item.id] : undefined}
		tabindex="-1"
		onclick={(e) => onEditClick(e, item)}
		ondblclick={(e) => onEditDouble(e, item)}
	>
		<span class="twisty">
			{#if item.children.length > 0}
				<Icon icon={collapsedEdits[item.id] ? ChevronRight : ChevronDown} size={12} />
			{/if}
		</span>
		<Icon icon={KIND_ICONS[item.kind]} size={14} />
		<span class="mono">{item.name}</span>
		{#if rule && !rule.unlock}
			<span class="lock-state small"><Icon icon={Lock} size={12} />Locked</span>
		{/if}
		<span class="spacer"></span>
		<span class="row-actions">
			{#if item.kind !== 'glob'}
				<button class="btn" onclick={() => openRule(item)}>Open</button>
			{/if}
			<button class="btn" onclick={pending(() => removeRules(item))}>Remove</button>
		</span>
	</div>
	{#if !collapsedEdits[item.id]}
		{#each item.children as child (child.id)}
			{@render editNode(child, depth + 1)}
		{/each}
	{/if}
{/snippet}

<div class="bar doc-toolbar">
	<span class="doc-title">Pack settings</span>
	<span class="mono small">modpack.json</span>
	<span class="spacer"></span>
	<button class="btn danger" onclick={removePack}>Remove pack…</button>
</div>

{#if manifest}
	<div class="settings">
		<nav class="nav">
			{#each SECTIONS as item (item.id)}
				<button
					class="nav-item"
					class:on={section === item.id}
					onclick={() => tabs.setArg(doc.id, item.id)}>{item.label}</button
				>
			{/each}
		</nav>
		<div class="scroll body">
			{#if section === 'general'}
				<div class="band bar">General</div>
				<label class="row">
					<span class="key">Name</span>
					<input
						class="field"
						value={manifest.name}
						onchange={(e) =>
							void change({ name: e.currentTarget.value.trim() || manifest.name }, false)}
					/>
				</label>
				<label class="row">
					<span class="key">Version</span>
					<input
						class="field mono"
						value={manifest.version}
						onchange={(e) =>
							void change({ version: e.currentTarget.value.trim() || '1.0.0' }, false)}
					/>
				</label>
				<label class="row">
					<span class="key">PACKWEAVE.md</span>
					<input
						type="checkbox"
						class="switch"
						checked={manifest.packweaveMd !== false}
						onchange={(e) => void change({ packweaveMd: e.currentTarget.checked }, true)}
					/>
					<span class="small hint">Auto-generated pack summary</span>
				</label>
				<div class="row">
					<span class="key">Icon</span>
					{#if ws.icon}<img class="icon" src={ws.icon} alt="" />{/if}
					<button class="btn" onclick={pickIcon}>Change…</button>
					{#if ws.icon}<button class="btn" onclick={clearIcon}>Remove</button>{/if}
				</div>
			{:else if section === 'game'}
				<div class="band bar">Game</div>
				<div class="row">
					<span class="key">Minecraft</span>
					<Select
						options={versions.map((v) => ({ value: v, label: v }))}
						bind:value={targetVersion}
						mono
					/>
					{#if targetVersion && targetVersion !== manifest.minecraft}
						<button class="btn" onclick={pending(() => switchMinecraft(targetVersion))}
							>Switch</button
						>
					{/if}
				</div>
				<div class="row">
					<span class="key">Loader</span>
					<Segmented
						options={[
							{ value: 'fabric', label: 'Fabric' },
							{ value: 'quilt', label: 'Quilt' },
							{ value: 'neoforge', label: 'NeoForge' },
							{ value: 'forge', label: 'Forge' },
							{ value: 'vanilla', label: 'Vanilla' },
						]}
						bind:value={loaderChoice}
						onchange={(v) => void switchLoader(v)}
					/>
				</div>
				{#if manifest.loader !== 'vanilla'}
					<div class="row">
						<span class="key">Loader version</span>
						<Select
							options={loaderVersionOptions(loaders, manifest.loaderVersion)}
							value={manifest.loaderVersion ?? ''}
							mono
							onchange={(v) => void change({ loaderVersion: v }, true)}
						/>
					</div>
				{/if}
				<div class="row">
					<span class="key">Data pack folder</span>
					<div class="field-slot">
						<PathField
							bind:value={datapackDraft}
							placeholder="not set"
							suggest={datapackHints}
							onsubmit={() => void setDatapacks()}
							onblur={() => void setDatapacks()}
						/>
					</div>
				</div>
				<div class="row">
					<span class="key">Versions</span>
					<Segmented
						options={[
							{ value: 'release' as Channel, label: 'Releases' },
							{ value: 'beta' as Channel, label: 'Betas' },
							{ value: 'alpha' as Channel, label: 'Alphas' },
						]}
						value={manifest.channel}
						onchange={(v) => void change({ channel: v }, false)}
					/>
				</div>
			{:else if section === 'instances'}
				<div class="band bar">Instances</div>
				<div class="thead">
					<span class="c1">Branch</span><span class="c2">Instance</span><span class="c3"></span>
				</div>
				{#each git.worktrees as tree (tree.path)}
					<div class="trow">
						<span class="c1 mono">{tree.branch ?? 'detached'}</span>
						<span class="c2">{instanceName(tree.instance)}</span>
						<span class="spacer"></span>
						{#if tree.branch === git.branch}
							<button class="btn" onclick={pending(() => linkInstance())}
								>{tree.instance ? 'Change…' : 'Link…'}</button
							>
						{/if}
					</div>
				{:else}
					<div class="trow">
						<span class="c1">{ws.name}</span>
						<span class="c2">{instance.name || 'none'}</span>
						<span class="spacer"></span>
						<button class="btn" onclick={pending(() => linkInstance())}
							>{instance.bound ? 'Change…' : 'Link…'}</button
						>
					</div>
				{/each}
				<div class="row">
					<span class="key">New branches</span>
					<Segmented
						options={[
							{ value: 'copy', label: "Copy main's" },
							{ value: 'move', label: "Move main's" },
							{ value: 'none', label: 'None' },
						]}
						value={packPrefs.newBranchInstance}
						onchange={(v) => app.updatePack(main, (p) => (p.newBranchInstance = v))}
					/>
				</div>
				<div class="row">
					<span class="key">Deleted branches</span>
					<Segmented
						options={[
							{ value: 'ask', label: 'Ask' },
							{ value: 'delete', label: 'Delete copy' },
							{ value: 'keep', label: 'Keep' },
						]}
						value={packPrefs.deletedBranchInstance}
						onchange={(v) => app.updatePack(main, (p) => (p.deletedBranchInstance = v))}
					/>
				</div>
			{:else if section === 'sources'}
				<div class="band bar">Sources</div>
				{#each app.providers as provider (provider.id)}
					<div class="row">
						<span class="key">{provider.displayName}</span>
						{#if provider.configured}<span class="tag added">✓ ready</span>{:else}<span
								class="tag removed">✗ not configured</span
							>{/if}
					</div>
				{/each}
			{:else if section === 'ignored'}
				{#each IGNORES as ignore (ignore.kind)}
					<div class="band bar">
						<span class="mono">{ignore.file}</span>
						<span class="small">{ignore.hint}</span>
						<span class="spacer"></span>
						<button class="btn" onclick={pending(() => resetIgnore(ignore.kind))}
							>Reset to defaults</button
						>
						<button
							class="btn"
							disabled={ignores[ignore.kind] === savedIgnores[ignore.kind]}
							onclick={pending(() => saveIgnore(ignore.kind))}
						>
							Save
						</button>
					</div>
					<div class="ignore-editor">
						<CodeEditor bind:value={ignores[ignore.kind]} lang="text" hints={ignoreHints} />
					</div>
				{/each}
			{:else if section === 'edits'}
				<div class="band bar">
					Player edits
					<span class="mono small">{PRESERVE_FILE}</span>
					<span class="spacer"></span>
					<button class="btn" onclick={pending(() => resetPreserve())}>Reset to defaults</button>
				</div>
				<div class="row small hint">
					In self-updating packs, player edits to these are preserved.
				</div>
				{#each editTree as item (item.id)}
					{@render editNode(item, 0)}
				{:else}
					<div class="row small hint">Nothing yet</div>
				{/each}
				<div class="row">
					<PathField
						bind:value={newPattern}
						placeholder="options.txt, config/ or config/mod.toml#setting"
						suggest={(text) => files.editHints(text)}
						onsubmit={addPattern}
					/>
					<button class="btn" disabled={!newPattern.trim()} onclick={addPattern}>Add</button>
				</div>
				<div class="row small hint">
					file#setting for one setting, ! in front to lock again. Each setting in a config file's
					Form view has a lock too.
				</div>
			{:else}
				<div class="band bar">Git and remote</div>
				{#if !git.isRepo}
					<div class="row">
						<button class="btn primary" onclick={pending(() => initRepo())}>Start using Git</button>
					</div>
				{:else}
					{#each git.remotes as remote (remote.name)}
						<div class="row">
							<span class="key mono">{remote.name}</span>
							<input
								class="field mono grow"
								value={remote.url}
								onchange={(e) =>
									void withGit(async () => {
										if (ws.dir) {
											await setRemoteUrl(ws.dir, remote.name, e.currentTarget.value.trim())
										}
										await git.refreshRefs()
									})}
							/>
							<button
								class="btn danger"
								onclick={pending(() =>
									withGit(async () => {
										if (ws.dir) await removeRemote(ws.dir, remote.name)
										await git.refreshRefs()
									}),
								)}
							>
								Remove
							</button>
						</div>
					{/each}
					<div class="row">
						<button class="btn" onclick={pending(() => addRemote())}>Add a remote…</button>
					</div>
					<div class="row small">Folder {shortenHome(ws.dir ?? '')}</div>
				{/if}
			{/if}
		</div>
	</div>
{/if}

<style>
	.settings {
		flex: 1;
		min-height: 0;
		display: flex;
	}

	.nav {
		flex: 0 0 180px;
		display: flex;
		flex-direction: column;
		padding: 8px 0;
		border-right: 1px solid var(--line);
	}

	.nav-item {
		height: var(--row);
		padding: 0 16px;
		text-align: left;
	}

	.nav-item:hover {
		background: var(--hover);
	}

	.nav-item.on {
		background: var(--selected);
		font-weight: 600;
	}

	.body {
		padding-bottom: 32px;
	}

	.row {
		display: flex;
		align-items: center;
		gap: 10px;
		min-height: 44px;
		padding: 6px 16px;
		border-bottom: 1px solid var(--line);
	}

	.key {
		flex: 0 0 160px;
		font-weight: 500;
	}

	.row .field {
		width: 320px;
	}

	.grow {
		flex: 1;
	}

	.icon {
		width: 48px;
		height: 48px;
		image-rendering: pixelated;
	}

	.c1 {
		flex: 0 0 200px;
	}

	.c2 {
		flex: 1;
	}

	.twisty {
		width: 12px;
		flex: none;
		display: inline-grid;
		place-items: center;
	}

	.lock-state {
		display: inline-flex;
		align-items: center;
		gap: 4px;
		color: var(--grey);
	}

	.ignore-editor {
		height: 240px;
		display: flex;
		border-bottom: 1px solid var(--line);
	}

	.field-slot {
		width: 320px;
		display: flex;
	}
</style>

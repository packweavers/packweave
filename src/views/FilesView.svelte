<script lang="ts">
	import { ChevronDown, ChevronRight, FilePlus, FolderPlus, RefreshCw } from '@lucide/svelte'
	import { tick, untrack } from 'svelte'

	import PromptDialog from '@/dialogs/PromptDialog.svelte'
	import { pathHints } from '@/lib/files/paths'
	import { openFile } from '@/lib/flows/files'
	import { changeMark, changeWord } from '@/lib/git/changes'
	import { fileMenu } from '@/lib/menus'
	import { content } from '@/lib/state/content.svelte'
	import { dialogs } from '@/lib/state/dialogs.svelte'
	import { files, OVERRIDES, PACK_FILES, shownAtRoot } from '@/lib/state/files.svelte'
	import { git } from '@/lib/state/git.svelte'
	import { menus } from '@/lib/state/menus.svelte'
	import { tabs } from '@/lib/state/tabs.svelte'
	import { ws } from '@/lib/state/workspace.svelte'
	import { join } from '@/lib/util/path'
	import Icon from '@/ui/Icon.svelte'
	import IconButton from '@/ui/IconButton.svelte'
	import { pending } from '@/ui/pending'

	import SideItem from './SideItem.svelte'
	import SideSection from './SideSection.svelte'

	let openMeta = $state(true)

	const rootEntries = $derived(files.listings[''] ?? [])
	const active = $derived(tabs.active?.kind === 'file' ? tabs.active.arg : null)
	const metaFiles = $derived([
		...PACK_FILES.filter(
			(name) => name === '.packweaveignore' || rootEntries.some((e) => e.name === name),
		),
		...rootEntries
			.filter((e) => !e.isDir && shownAtRoot(e.name) && !PACK_FILES.includes(e.name))
			.map((e) => e.name),
	])

	$effect(() => {
		if (!ws.dir) return
		void files.list(OVERRIDES)
		void files.list('')
	})

	$effect(() => {
		void git.status
		void content.history
		untrack(() => void files.refresh())
	})

	$effect(() => {
		const focus = files.focus
		if (!focus) return
		if (!focus.rel.includes('/')) openMeta = true
		void tick().then(() =>
			document.getElementById(`files-${focus.rel}`)?.scrollIntoView({ block: 'nearest' }),
		)
	})

	function changesUnder(dir: string): number {
		return git.changes.filter((c) => c.path.startsWith(`${dir}/`)).length
	}

	function open(rel: string, keep: boolean): void {
		files.focus = null
		openFile(rel, !keep)
	}

	function toggle(rel: string): void {
		files.focus = null
		files.toggle(rel)
	}

	async function create(folder: boolean): Promise<void> {
		const target =
			active && files.listings[active] === undefined
				? active.slice(0, active.lastIndexOf('/'))
				: OVERRIDES
		const base = `${target || OVERRIDES}/`
		const name = await dialogs.open(PromptDialog, {
			title: folder ? 'New folder' : 'New file',
			label: 'Name',
			placeholder: folder ? 'config' : 'config/my-mod.toml',
			confirm: 'Create',
			suggest: async (text: string) =>
				pathHints(
					(await files.packPaths())
						.filter((p) => p.endsWith('/') && p.startsWith(base))
						.map((p) => p.slice(base.length)),
					text,
				),
		})
		if (!name) return
		const rel = await files.create(target || OVERRIDES, name, folder)
		if (rel && !folder) open(rel, true)
	}

	async function rename(rel: string): Promise<void> {
		const name = await dialogs.open(PromptDialog, {
			title: 'Rename',
			label: 'Name',
			value: rel.slice(rel.lastIndexOf('/') + 1),
			confirm: 'Rename',
		})
		if (!name) return
		const next = await files.rename(rel, name)
		if (!next) return
		for (const doc of tabs.all) {
			if (doc.kind !== 'file' || (doc.arg !== rel && !doc.arg.startsWith(`${rel}/`))) continue
			tabs.rekey(doc.id, next + doc.arg.slice(rel.length))
		}
	}

	function contextMenu(event: MouseEvent, rel: string, isDir: boolean): void {
		menus.at(event, [
			...fileMenu(rel, isDir, true),
			'separator',
			{ label: 'Rename…', run: () => void rename(rel) },
		])
	}
</script>

{#snippet tree(dir: string, depth: number)}
	{#each files.listings[dir] ?? [] as entry (entry.name)}
		{@const rel = join(dir, entry.name)}
		{#if entry.isDir}
			{@const count = changesUnder(rel)}
			<SideItem
				id="files-{rel}"
				label={entry.name}
				{depth}
				active={files.focus?.rel === rel}
				count={count || null}
				onclick={() => toggle(rel)}
				oncontextmenu={(e) => contextMenu(e, rel, true)}
			>
				{#snippet lead()}<Icon
						icon={files.expanded[rel] ? ChevronDown : ChevronRight}
						size={12}
					/>{/snippet}
			</SideItem>
			{#if files.expanded[rel]}
				{@render tree(rel, depth + 1)}
			{/if}
		{:else}
			{@const change = git.changes.find((c) => c.path === rel)}
			<SideItem
				id="files-{rel}"
				label={entry.name}
				{depth}
				active={active === rel}
				onclick={() => open(rel, false)}
				ondblclick={() => open(rel, true)}
				oncontextmenu={(e) => contextMenu(e, rel, false)}
			>
				{#snippet lead()}<span class="spacer-12"></span>{/snippet}
				{#snippet trailing()}
					{#if change}
						{@const word = changeWord(change)}
						<span class="mark-text {word}">{changeMark(word)}</span>
					{/if}
					{#if files.isDirty(rel)}<span class="dirty"></span>{/if}
				{/snippet}
			</SideItem>
		{/if}
	{/each}
{/snippet}

<div class="bar view-header">
	<span>Files</span>
	<span class="spacer"></span>
	<IconButton icon={FilePlus} label="New file…" onclick={pending(() => create(false))} />
	<IconButton icon={FolderPlus} label="New folder…" onclick={pending(() => create(true))} />
	<IconButton icon={RefreshCw} label="Refresh" onclick={pending(() => files.refresh())} />
</div>
<div class="scroll">
	<div class="root">
		{#if (files.listings[OVERRIDES] ?? []).length === 0}
			<div class="quiet small">No files yet</div>
		{/if}
		{@render tree(OVERRIDES, 0)}
	</div>
	<SideSection title="Pack metadata" bind:open={openMeta}>
		{#each metaFiles as name (name)}
			{@const change = git.changes.find((c) => c.path === name)}
			<SideItem
				id="files-{name}"
				label={name}
				active={active === name}
				onclick={() => open(name, false)}
				ondblclick={() => open(name, true)}
				oncontextmenu={(e) => contextMenu(e, name, false)}
			>
				{#snippet trailing()}
					{#if change}
						{@const word = changeWord(change)}
						<span class="mark-text {word}">{changeMark(word)}</span>
					{/if}
				{/snippet}
			</SideItem>
		{/each}
	</SideSection>
</div>

<style>
	.root {
		padding: 4px 0;
	}

	.quiet {
		padding: 6px 16px;
	}

	.spacer-12 {
		width: 12px;
		flex: none;
	}

	.dirty {
		width: 8px;
		height: 8px;
		border-radius: 50%;
		background: var(--text);
		flex: none;
	}
</style>

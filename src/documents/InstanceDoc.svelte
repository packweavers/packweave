<script lang="ts">
	import { ChevronDown, ChevronRight } from '@lucide/svelte'

	import { buildTree, leaves, type TreeNode } from '@/lib/files/tree'
	import { openIgnoreFile, showInFiles } from '@/lib/flows/files'
	import { linkInstance } from '@/lib/flows/instance'
	import {
		coversKey,
		folderEntry,
		ignoreEntry,
		pausedWhileRunning,
		SIDE_WORDS,
		type SyncItem,
	} from '@/lib/instance/items'
	import { bisect } from '@/lib/state/bisect.svelte'
	import { content } from '@/lib/state/content.svelte'
	import { instance } from '@/lib/state/instance.svelte'
	import { menus } from '@/lib/state/menus.svelte'
	import { selection } from '@/lib/state/selection.svelte'
	import { tabs, type Doc } from '@/lib/state/tabs.svelte'
	import { toasts } from '@/lib/state/toasts.svelte'
	import { ws } from '@/lib/state/workspace.svelte'
	import { addIgnorePatterns } from '@/lib/tauri/instance'
	import type { SyncIgnored } from '@/lib/tauri/types'
	import { reveal } from '@/lib/tauri/windows'
	import { ago, plural } from '@/lib/util/format'
	import { join } from '@/lib/util/path'
	import { fileManager } from '@/lib/util/platform'
	import Empty from '@/ui/Empty.svelte'
	import Icon from '@/ui/Icon.svelte'
	import { pending } from '@/ui/pending'

	interface Props {
		doc: Doc
	}

	let { doc }: Props = $props()

	let unchecked = $state<Record<string, boolean>>({})
	let collapsed = $state<Record<string, boolean>>({})
	let showIgnored = $state(false)

	const selected = $derived(selection.of(doc.id)[0] ?? null)
	const running = $derived(instance.running !== null)
	const pushChecked = $derived(instance.toPush.filter((i) => !unchecked[i.key] && !paused(i)))
	const pullChecked = $derived(instance.toPull.filter((i) => !unchecked[i.key] && !paused(i)))
	const syncCount = $derived(pushChecked.length + pullChecked.length)
	const ignoredRows = $derived(
		instance.ignored.map((entry) => ({
			entry,
			items: instance.items.filter((i) => i.ignored && coversKey(entry, i.key)),
		})),
	)

	function check(items: SyncItem[], on: boolean): void {
		for (const item of items) {
			if (on) {
				delete unchecked[item.key]
			} else {
				unchecked[item.key] = true
			}
		}
	}

	function paused(item: SyncItem): boolean {
		return running && item.side !== 'both' && pausedWhileRunning(item, item.side)
	}

	function checkState(items: SyncItem[]): 'all' | 'some' | 'none' {
		const on = items.filter((i) => !unchecked[i.key]).length
		return on === items.length ? 'all' : on === 0 ? 'none' : 'some'
	}

	function gitignore(item: SyncItem): void {
		const dir = ws.dir
		if (!dir || !item.path) return
		void addIgnorePatterns(dir, 'git', [`overrides/${item.path}`])
			.then(() => {
				ws.changed()
				void instance.scan()
			})
			.catch((error) => toasts.error(error))
	}

	function act(event: MouseEvent, action: () => Promise<unknown>): void {
		event.stopPropagation()
		void action()
	}

	function syncChecked(): void {
		const items = [...pushChecked, ...pullChecked]
		const directions = items.map((i) => (i.side === 'push' ? ('push' as const) : ('pull' as const)))
		void instance.apply(
			items,
			directions,
			`Syncing ${instance.name}`,
			(n) => `Synced ${n} with ${instance.name}`,
		)
	}

	function rowMenu(item: SyncItem, event: MouseEvent): void {
		menus.at(event, [
			...(item.side !== 'pull' ? [{ label: 'Push', run: () => void instance.push([item]) }] : []),
			...(item.side !== 'push' ? [{ label: 'Pull', run: () => void instance.pull([item]) }] : []),
			...(item.side !== 'both'
				? [{ label: 'Revert', run: () => void instance.revert([item]) }]
				: []),
			'separator',
			{
				label: `Ignore for ${instance.name}`,
				run: () => void instance.ignore([ignoreEntry(item)]),
			},
			...(item.path ? [{ label: 'Add to .gitignore', run: () => gitignore(item) }] : []),
		])
	}

	function folderMenu(path: string, event: MouseEvent): void {
		menus.at(event, [
			{
				label: `Ignore folder for ${instance.name}`,
				run: () => void instance.ignore([folderEntry(path)]),
			},
		])
	}

	function ignoredMenu(entry: SyncIgnored, event: MouseEvent): void {
		menus.at(event, [{ label: 'Stop ignoring', run: () => void instance.unignore([entry.key]) }])
	}

	function ignoredState(items: SyncItem[]): string {
		if (items.length === 0) return 'same on both sides'
		if (items.length > 1) return plural(items.length, 'difference', 'differences')
		return `${items[0].change} · ${SIDE_WORDS[items[0].side]}`
	}

	async function showPath(path: string, isDir: boolean): Promise<void> {
		if (await showInFiles(`overrides/${path}`, isDir)) return
		const game = instance.binding?.gameDir
		if (!game) return
		toasts.info(`Only in ${instance.name}`, [
			{ label: `Show in ${fileManager}`, run: () => void reveal(join(game, path)) },
		])
	}

	function canOpen(item: SyncItem): boolean {
		if (item.path || item.key.startsWith('jar:')) return true
		return item.key.startsWith('mod:') && !!content.find(item.key.slice('mod:'.length))
	}

	function showItem(event: MouseEvent, item: SyncItem): void {
		if ((event.target as Element).closest('button, input')) return
		openItem(item)
	}

	function openItem(item: SyncItem): void {
		if (item.path) {
			void showPath(item.path, false)
		} else if (item.key.startsWith('jar:')) {
			void showPath(item.key.slice('jar:'.length), false)
		} else if (item.key.startsWith('mod:')) {
			const id = item.key.slice('mod:'.length)
			if (content.find(id)) tabs.open('mod', id)
		}
	}

	function showFolder(event: MouseEvent, path: string): void {
		if ((event.target as Element).closest('input, .row-actions')) return
		void showPath(path, true)
	}

	function toggleFolder(event: MouseEvent, key: string): void {
		if ((event.target as Element).closest('input, .row-actions')) return
		collapsed[key] = !collapsed[key]
	}

	function fileTree(items: SyncItem[]): TreeNode<SyncItem>[] {
		return buildTree(
			items.filter((i) => i.path),
			(i) => i.path!,
		)
	}
</script>

{#snippet row(item: SyncItem, depth: number, name: string)}
	<div
		class="trow"
		class:selected={selected === item.key}
		style:padding-left="{16 + depth * 16}px"
		role="option"
		aria-selected={selected === item.key}
		tabindex="-1"
		onclick={() => selection.set(doc.id, [item.key])}
		ondblclick={(e) => showItem(e, item)}
		oncontextmenu={(e) => rowMenu(item, e)}
	>
		<span class="twisty"></span>
		{#if item.side !== 'both'}
			<input
				type="checkbox"
				checked={!unchecked[item.key] && !paused(item)}
				disabled={paused(item)}
				onclick={(e) => e.stopPropagation()}
				onchange={(e) => check([item], e.currentTarget.checked)}
			/>
		{/if}
		<span class="name">{name}</span>
		<span class="tag {item.change}">{item.change}</span>
		<span class="mono small">{item.detail}</span>
		<span class="spacer"></span>
		{#if item.modified}<span class="small">{ago(item.modified * 1000, 'short')}</span>{/if}
		<span class="row-actions">
			{#if canOpen(item)}
				<button
					class="btn"
					onclick={(e) => {
						e.stopPropagation()
						openItem(item)
					}}>Open</button
				>
			{/if}
			{#if item.side === 'both'}
				<button class="btn" onclick={(e) => act(e, () => instance.push([item]))}>Push</button>
				<button class="btn" onclick={(e) => act(e, () => instance.pull([item]))}>Pull</button>
			{:else}
				<button class="btn" onclick={(e) => act(e, () => instance.revert([item]))}>Revert</button>
			{/if}
		</span>
	</div>
{/snippet}

{#snippet folder(node: TreeNode<SyncItem>, depth: number, side: string)}
	{#if node.item}
		{@render row(node.item, depth, node.name)}
	{:else}
		{@const items = leaves(node.children)}
		{@const s = checkState(items)}
		{@const key = `${side}:${node.path}`}
		<div
			class="trow folder"
			style:padding-left="{16 + depth * 16}px"
			role="treeitem"
			aria-selected="false"
			aria-expanded={!collapsed[key]}
			tabindex="-1"
			onclick={(e) => toggleFolder(e, key)}
			ondblclick={(e) => showFolder(e, node.path)}
			oncontextmenu={(e) => folderMenu(node.path, e)}
		>
			<span class="twisty"
				><Icon icon={collapsed[key] ? ChevronRight : ChevronDown} size={12} /></span
			>
			{#if items.every((i) => i.side !== 'both')}
				<input
					type="checkbox"
					checked={s === 'all'}
					indeterminate={s === 'some'}
					onchange={() => check(items, s !== 'all')}
				/>
			{/if}
			<span class="name">{node.name}/</span>
			<span class="small">{plural(node.count, 'file', 'files')}</span>
			<span class="spacer"></span>
			<span class="row-actions">
				<button class="btn" onclick={(e) => act(e, () => showPath(node.path, true))}>Open</button>
				{#if items.every((i) => i.side === 'both')}
					<button class="btn" onclick={(e) => act(e, () => instance.push(items))}>Push</button>
					<button class="btn" onclick={(e) => act(e, () => instance.pull(items))}>Pull</button>
				{:else}
					<button class="btn" onclick={(e) => act(e, () => instance.revert(items))}>Revert</button>
				{/if}
			</span>
		</div>
		{#if !collapsed[key]}
			{#each node.children as child (child.path)}
				{@render folder(child, depth + 1, side)}
			{/each}
		{/if}
	{/if}
{/snippet}

{#snippet section(title: string, items: SyncItem[], side: 'push' | 'pull' | 'both')}
	{#if items.length > 0}
		{@const s = checkState(items)}
		{@const checked = items.filter((i) => !unchecked[i.key])}
		<div class="band bar">
			{#if side !== 'both'}
				<input
					type="checkbox"
					checked={s === 'all'}
					indeterminate={s === 'some'}
					onchange={() => check(items, s !== 'all')}
				/>
			{/if}
			{title}
			<span class="count-text">{items.length}</span>
			<span class="spacer"></span>
			{#if side !== 'both'}
				<button
					class="btn"
					disabled={checked.length === 0 || instance.applying}
					onclick={pending(() => instance.revert(checked))}>Revert</button
				>
				<button
					class="btn"
					disabled={checked.length === 0 || instance.applying}
					onclick={pending(() =>
						side === 'push' ? instance.push(checked) : instance.pull(checked),
					)}
				>
					{side === 'push' ? 'Push' : 'Pull'}
					{checked.length}
				</button>
			{/if}
		</div>
		{#each items.filter((i) => !i.path) as item (item.key)}
			{@render row(item, 0, item.label)}
		{/each}
		{#each fileTree(items) as node (node.path)}
			{@render folder(node, 0, side)}
		{/each}
	{/if}
{/snippet}

{#snippet ignoredSection()}
	{#if ignoredRows.length > 0}
		<button class="band bar toggle" onclick={() => (showIgnored = !showIgnored)}>
			<Icon icon={showIgnored ? ChevronDown : ChevronRight} size={14} />
			Ignored
			<span class="count-text">{ignoredRows.length}</span>
			<span class="spacer"></span>
			<span class="small hint">only for {instance.name}</span>
		</button>
		{#if showIgnored}
			{#each ignoredRows as { entry, items } (entry.key)}
				<div
					class="trow"
					class:selected={selected === entry.key}
					role="option"
					aria-selected={selected === entry.key}
					tabindex="-1"
					onclick={() => selection.set(doc.id, [entry.key])}
					oncontextmenu={(e) => ignoredMenu(entry, e)}
				>
					<span class="name" class:mono={entry.key.startsWith('file:')}>{entry.label}</span>
					<span class="spacer"></span>
					<span class="small hint">{ignoredState(items)}</span>
					<span class="row-actions">
						<button class="btn" onclick={(e) => act(e, () => instance.unignore([entry.key]))}
							>Stop ignoring</button
						>
					</span>
				</div>
			{/each}
		{/if}
	{/if}
{/snippet}

{#if !instance.bound}
	<Empty title="No instance linked">
		<button class="btn primary" onclick={() => void linkInstance()}>Link…</button>
	</Empty>
{:else}
	<div class="bar doc-toolbar">
		<span class="doc-title">{instance.name}</span>
		<span class="small">{instance.binding?.launcher}</span>
		<span class="spacer"></span>
		<button class="btn" onclick={() => void openIgnoreFile('git')}>Edit .gitignore</button>
		<button
			class="btn"
			disabled={instance.scanning}
			aria-busy={instance.scanning}
			onclick={() => void instance.scan()}>{instance.scanning ? 'Scanning…' : 'Rescan'}</button
		>
		<button
			class="btn primary"
			disabled={syncCount === 0 || instance.applying || bisect.active}
			aria-busy={instance.applying}
			onclick={syncChecked}
		>
			Sync {syncCount}
		</button>
	</div>
	{#if bisect.active}
		<div class="banner warn">
			Find a broken mod has mods turned off in {instance.name} · Sync paused
			<span class="spacer"></span>
			<button class="btn" onclick={() => tabs.open('bisect')}>Open</button>
		</div>
	{/if}
	{#if instance.running}
		<div class="banner warn">
			{instance.name} is running · Mods paused
		</div>
	{/if}
	{#if instance.missing}
		<div class="banner error">
			Instance folder missing
			<span class="spacer"></span>
			<button class="btn" onclick={() => void linkInstance()}>Fix…</button>
		</div>
	{/if}
	{#if instance.report && instance.active.length === 0 && ignoredRows.length === 0}
		<Empty title="In sync" />
	{:else}
		<div class="scroll">
			{#if instance.report && instance.active.length === 0}
				<div class="in-sync">In sync</div>
			{/if}
			{@render section('To push', instance.toPush, 'push')}
			{@render section('To pull', instance.toPull, 'pull')}
			{@render section('Changed on both sides', instance.both, 'both')}
			{@render ignoredSection()}
		</div>
	{/if}
{/if}

<style>
	.folder {
		font-weight: 500;
	}

	.twisty {
		width: 12px;
		flex: none;
		display: inline-grid;
		place-items: center;
	}

	.toggle {
		width: 100%;
		text-align: left;
	}

	.in-sync {
		padding: 24px 16px;
		color: var(--grey);
		text-align: center;
	}
</style>

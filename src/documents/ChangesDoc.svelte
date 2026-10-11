<script lang="ts">
	import { ChevronDown, ChevronRight } from '@lucide/svelte'
	import { untrack } from 'svelte'

	import { contentChange } from '@/lib/content/changes'
	import { buildTree, leaves, type TreeNode } from '@/lib/files/tree'
	import { openInFiles } from '@/lib/flows/files'
	import { discard, discardContent } from '@/lib/flows/git'
	import {
		changeMark,
		changeWord,
		displayPath,
		folderPathOf,
		isContentPath,
	} from '@/lib/git/changes'
	import { contentChangeMenu, fileMenu } from '@/lib/menus'
	import { content } from '@/lib/state/content.svelte'
	import { git } from '@/lib/state/git.svelte'
	import { menus } from '@/lib/state/menus.svelte'
	import { tabs, type Doc } from '@/lib/state/tabs.svelte'
	import { toasts } from '@/lib/state/toasts.svelte'
	import { ws } from '@/lib/state/workspace.svelte'
	import { diffFile } from '@/lib/tauri/git'
	import type { GitChange } from '@/lib/tauri/types'
	import { plural } from '@/lib/util/format'
	import DiffView from '@/ui/DiffView.svelte'
	import Empty from '@/ui/Empty.svelte'
	import Icon from '@/ui/Icon.svelte'
	import { pending } from '@/ui/pending'
	import Segmented from '@/ui/Segmented.svelte'

	interface Props {
		doc: Doc
	}

	let {}: Props = $props()

	let layoutMode = $state<'unified' | 'split'>('unified')
	let expanded = $state<Record<string, boolean>>({})
	let collapsed = $state<Record<string, boolean>>({})
	let diffs = $state<Record<string, string>>({})

	const fileChanges = $derived(git.changes.filter((c) => !isContentPath(c.path)))
	const tree = $derived(buildTree(fileChanges, (c) => displayPath(c.path)))
	const items = $derived(content.changes?.items ?? [])

	$effect(() => {
		const target = git.focus
		if (!target) return
		expanded[target] = true
		void load(target)
		requestAnimationFrame(() =>
			document.getElementById(`change-${target}`)?.scrollIntoView({ block: 'start' }),
		)
	})

	$effect(() => {
		void git.status
		untrack(reloadExpanded)
	})

	function reloadExpanded(): void {
		for (const [path, open] of Object.entries(expanded)) {
			if (open) void load(path, false)
		}
	}

	async function load(path: string, loud = true): Promise<void> {
		if (!ws.dir) return
		diffs[path] = await diffFile(ws.dir, path).catch((error) => {
			if (loud) toasts.error(error)
			return ''
		})
	}

	function toggle(path: string): void {
		expanded[path] = !expanded[path]
		if (expanded[path]) void load(path)
	}

	function expandAll(): void {
		for (const change of fileChanges) {
			expanded[change.path] = true
			void load(change.path)
		}
	}

	function showFile(event: MouseEvent, path: string): void {
		if ((event.target as Element).closest('.twisty, .row-actions')) return
		void openInFiles(path, false)
	}

	function toggleFolder(event: MouseEvent, path: string): void {
		if ((event.target as Element).closest('.row-actions')) return
		collapsed[path] = !collapsed[path]
	}

	function showFolder(event: MouseEvent, node: TreeNode<GitChange>): void {
		if ((event.target as Element).closest('.row-actions')) return
		void openInFiles(folderPathOf(node), true)
	}

	function stats(node: TreeNode<GitChange>): { added: number; removed: number } {
		const files = node.item ? [node.item] : leaves(node.children)
		return files.reduce(
			(sum, c) => ({
				added: sum.added + (git.stats[c.path]?.added ?? 0),
				removed: sum.removed + (git.stats[c.path]?.removed ?? 0),
			}),
			{ added: 0, removed: 0 },
		)
	}
</script>

{#snippet statText(node: TreeNode<GitChange>)}
	{@const s = stats(node)}
	<span class="mono small">{s.added ? `+${s.added}` : ''} {s.removed ? `−${s.removed}` : ''}</span>
{/snippet}

{#snippet fileNode(node: TreeNode<GitChange>, depth: number)}
	{#if node.item}
		{@const w = changeWord(node.item)}
		<div
			class="trow"
			id="change-{node.item.path}"
			style:padding-left="{16 + depth * 16}px"
			ondblclick={(e) => showFile(e, node.item!.path)}
			oncontextmenu={(e) => menus.at(e, fileMenu(node.item!.path, false))}
			role="row"
			tabindex="-1"
		>
			<button class="twisty" onclick={() => toggle(node.item!.path)} aria-label="Show diff">
				<Icon icon={expanded[node.item.path] ? ChevronDown : ChevronRight} size={12} />
			</button>
			<span class="mark-text {w}">{changeMark(w)}</span>
			<button class="name link" onclick={() => tabs.open('file', node.item!.path)}
				>{node.name}</button
			>
			{@render statText(node)}
			<span class="spacer"></span>
			<span class="row-actions">
				<button class="btn" onclick={() => tabs.open('file', node.item!.path)}>Open</button>
				<button class="btn danger" onclick={pending(() => discard([node.item!.path]))}
					>Discard…</button
				>
			</span>
		</div>
		{#if expanded[node.item.path]}
			<div class="diff-box">
				<DiffView diff={diffs[node.item.path] ?? ''} split={layoutMode === 'split'} />
			</div>
		{/if}
	{:else}
		<div
			class="trow folder"
			style:padding-left="{16 + depth * 16}px"
			role="treeitem"
			aria-selected="false"
			aria-expanded={!collapsed[node.path]}
			tabindex="-1"
			onclick={(e) => toggleFolder(e, node.path)}
			ondblclick={(e) => showFolder(e, node)}
			oncontextmenu={(e) => menus.at(e, fileMenu(folderPathOf(node), true))}
		>
			<span class="twisty"
				><Icon icon={collapsed[node.path] ? ChevronRight : ChevronDown} size={12} /></span
			>
			<span class="name">{node.name}/</span>
			<span class="small">{plural(node.count, 'file', 'files')}</span>
			{@render statText(node)}
			<span class="spacer"></span>
			<span class="row-actions">
				<button class="btn" onclick={() => void openInFiles(folderPathOf(node), true)}>Open</button>
				<button
					class="btn danger"
					onclick={pending(() => discard(leaves(node.children).map((c) => c.path)))}
					>Discard…</button
				>
			</span>
		</div>
		{#if !collapsed[node.path]}
			{#each node.children as child (child.path)}
				{@render fileNode(child, depth + 1)}
			{/each}
		{/if}
	{/if}
{/snippet}

<div class="bar doc-toolbar">
	<span class="doc-title">Changes</span>
	<span class="spacer"></span>
	<Segmented
		options={[
			{ value: 'unified', label: 'Unified' },
			{ value: 'split', label: 'Side by side' },
		]}
		bind:value={layoutMode}
	/>
	<button
		class="btn danger"
		disabled={git.changes.length === 0}
		onclick={pending(() => discard(git.changes.map((c) => c.path)))}
	>
		Discard…
	</button>
</div>

{#if git.changes.length === 0}
	<Empty title="No changes" />
{:else}
	<div class="scroll">
		{#if items.length > 0}
			<div class="band bar">Content <span class="count-text">{items.length}</span></div>
			{#each items as item (item.path)}
				{@const w = contentChange(item.kind)}
				<div
					class="trow"
					role="row"
					tabindex="-1"
					ondblclick={(e) => {
						if ((e.target as Element).closest('.row-actions') || item.kind === 'removed') return
						tabs.open('mod', item.path)
					}}
					oncontextmenu={(e) => menus.at(e, contentChangeMenu(item))}
				>
					<span class="mark {w}"></span>
					<span class="twisty"></span>
					<span class="mark-text {w}">{changeMark(w)}</span>
					<span class="name">{item.name}</span>
					{#if item.dependency}<span class="tag">dependency</span>{/if}
					{#if item.kind === 'disabled' || item.kind === 'enabled'}<span class="tag"
							>{item.kind}</span
						>{/if}
					<span class="mono small">
						{#if item.kind === 'updated'}{item.fromVersion} → {item.toVersion}{:else}{item.toVersion ??
								item.fromVersion ??
								''}{/if}
					</span>
					<span class="spacer"></span>
					<span class="row-actions">
						{#if item.kind === 'removed'}
							<button class="btn" onclick={pending(() => discardContent([item], false))}
								>Restore</button
							>
						{:else}
							<button class="btn" onclick={() => tabs.open('mod', item.path)}>Open</button>
							<button class="btn danger" onclick={pending(() => discardContent([item]))}
								>Discard…</button
							>
						{/if}
					</span>
				</div>
			{/each}
		{/if}
		{#if fileChanges.length > 0}
			<div class="band bar">
				Files <span class="count-text">{fileChanges.length}</span>
				<span class="spacer"></span>
				<button class="btn" onclick={expandAll}>Expand all</button>
			</div>
			{#each tree as node (node.path)}
				{@render fileNode(node, 0)}
			{/each}
		{/if}
	</div>
{/if}

<style>
	.twisty {
		width: 16px;
		display: inline-grid;
		place-items: center;
		flex: none;
	}

	.folder {
		font-weight: 500;
	}

	.link:hover {
		text-decoration: underline;
	}

	.diff-box {
		border-bottom: 1px solid var(--line);
		overflow-x: auto;
	}
</style>

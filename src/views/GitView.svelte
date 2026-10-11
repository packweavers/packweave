<script lang="ts">
	import {
		ArrowDownToLine,
		ArrowUpFromLine,
		ChevronDown,
		ChevronRight,
		Ellipsis,
		History,
		RefreshCw,
	} from '@lucide/svelte'
	import { onMount } from 'svelte'

	import { contentChange } from '@/lib/content/changes'
	import { buildTree, leaves, type TreeNode } from '@/lib/files/tree'
	import { abortMerge, commitMerge, newBranch } from '@/lib/flows/branches'
	import { openInFiles } from '@/lib/flows/files'
	import {
		applyStash,
		commit,
		dropStash,
		fetch,
		initRepo,
		pull,
		push,
		stashChanges,
	} from '@/lib/flows/git'
	import {
		changeMark,
		changeWord,
		displayPath,
		folderPathOf,
		isContentPath,
	} from '@/lib/git/changes'
	import { branchMenu, contentChangeMenu, fileMenu, tagMenu } from '@/lib/menus'
	import { comboFor, keysFor } from '@/lib/state/commands.svelte'
	import { content } from '@/lib/state/content.svelte'
	import { git } from '@/lib/state/git.svelte'
	import { comboOf, normalizeCombo } from '@/lib/state/keymap'
	import { menus } from '@/lib/state/menus.svelte'
	import { tabs } from '@/lib/state/tabs.svelte'
	import type { GitChange } from '@/lib/tauri/types'
	import Empty from '@/ui/Empty.svelte'
	import Icon from '@/ui/Icon.svelte'
	import IconButton from '@/ui/IconButton.svelte'
	import { pending } from '@/ui/pending'

	import SideItem from './SideItem.svelte'
	import SideSection from './SideSection.svelte'

	let open = $state({ changes: true, branches: true, remotes: false, tags: false, stashes: false })
	let collapsed = $state<Record<string, boolean>>({})
	let amend = $state(false)
	let committing = $state(false)

	const fileChanges = $derived(git.changes.filter((c) => !isContentPath(c.path)))
	const contentChanges = $derived(content.changes?.items ?? [])
	const tree = $derived(buildTree(fileChanges, (c) => displayPath(c.path)))
	const included = $derived(git.included)
	const total = $derived(git.changes.filter((c) => !c.conflicted).length)
	const ahead = $derived(git.status?.ahead ?? 0)
	const behind = $derived(git.status?.behind ?? 0)

	onMount(() => {
		void git.refreshAll()
	})

	function checkedState(node: TreeNode<GitChange>): 'all' | 'some' | 'none' {
		const items = node.item ? [node.item] : leaves(node.children)
		const on = items.filter((c) => !git.excluded[c.path]).length
		return on === items.length ? 'all' : on === 0 ? 'none' : 'some'
	}

	function toggleNode(node: TreeNode<GitChange>): void {
		const items = node.item ? [node.item] : leaves(node.children)
		git.include(
			items.map((c) => c.path),
			checkedState(node) !== 'all',
		)
	}

	function openChange(change: GitChange): void {
		git.focus = change.path
		tabs.open('changes', '', { preview: true })
	}

	async function submit(): Promise<void> {
		if (!git.message.trim() || (included.length === 0 && !amend)) return
		committing = true
		try {
			if (await commit(git.message.trim(), included, amend)) {
				git.message = ''
				amend = false
			}
		} finally {
			committing = false
		}
	}

	function onmessagekey(event: KeyboardEvent): void {
		const combo = comboOf(event)
		if (combo && combo === normalizeCombo(comboFor('git.commit') ?? 'Mod+Enter')) {
			event.preventDefault()
			void submit()
		}
	}

	function gitMenu(event: MouseEvent): void {
		menus.below(event.currentTarget as Element, [
			{ label: 'Pull', command: 'git.pull', disabled: !git.remoteName, run: () => void pull() },
			{ label: 'Push', command: 'git.push', disabled: !git.remoteName, run: () => void push() },
			{ label: 'Fetch', disabled: !git.remoteName, run: () => void fetch() },
			'separator',
			{ label: 'New branch…', command: 'branch.new', run: () => void newBranch() },
			{
				label: 'Stash changes…',
				disabled: git.changes.length === 0,
				run: () => void stashChanges(),
			},
			'separator',
			{ label: 'Branches', run: () => tabs.open('branches') },
			{ label: 'History', run: () => tabs.open('history') },
		])
	}
</script>

{#snippet node(item: TreeNode<GitChange>, depth: number)}
	{@const checked = checkedState(item)}
	{#if item.item}
		{@const word = changeWord(item.item)}
		{@const stat = git.stats[item.item.path]}
		<SideItem
			label={item.name}
			{depth}
			onclick={() => openChange(item.item!)}
			ondblclick={() => void openInFiles(item.item!.path, false)}
			oncontextmenu={(e) => menus.at(e, fileMenu(item.item!.path, false))}
		>
			{#snippet lead()}
				<span class="twisty"></span>
				<input
					type="checkbox"
					checked={checked === 'all'}
					onclick={(e) => e.stopPropagation()}
					ondblclick={(e) => e.stopPropagation()}
					onchange={() => toggleNode(item)}
				/>
			{/snippet}
			{#snippet trailing()}
				{#if stat && (stat.added || stat.removed)}
					<span class="mono small"
						>{stat.added ? `+${stat.added}` : ''} {stat.removed ? `−${stat.removed}` : ''}</span
					>
				{/if}
				<span class="mark-text {word}">{changeMark(word)}</span>
			{/snippet}
		</SideItem>
	{:else}
		<SideItem
			label="{item.name}/"
			{depth}
			count={item.count}
			onclick={() => (collapsed[item.path] = !collapsed[item.path])}
			ondblclick={() => void openInFiles(folderPathOf(item), true)}
			oncontextmenu={(e) => menus.at(e, fileMenu(folderPathOf(item), true))}
		>
			{#snippet lead()}
				<span class="twisty"
					><Icon icon={collapsed[item.path] ? ChevronRight : ChevronDown} size={12} /></span
				>
				<input
					type="checkbox"
					checked={checked === 'all'}
					indeterminate={checked === 'some'}
					onclick={(e) => e.stopPropagation()}
					ondblclick={(e) => e.stopPropagation()}
					onchange={() => toggleNode(item)}
				/>
			{/snippet}
		</SideItem>
		{#if !collapsed[item.path]}
			{#each item.children as child (child.path)}
				{@render node(child, depth + 1)}
			{/each}
		{/if}
	{/if}
{/snippet}

<div class="bar view-header">
	<span>Git</span>
	<span class="spacer"></span>
	{#if git.isRepo}
		<IconButton icon={RefreshCw} label="Refresh" onclick={pending(() => git.refreshAll())} />
		<IconButton icon={History} label="History" onclick={() => tabs.open('history')} />
		<IconButton icon={Ellipsis} label="More" onclick={gitMenu} />
	{/if}
</div>

{#if git.status && !git.isRepo}
	<Empty title="Not using Git yet">
		<button class="btn primary" onclick={pending(initRepo)}>Start using Git</button>
	</Empty>
{:else}
	<div class="scroll">
		{#if git.merging}
			<div class="banner warn">
				Merging {git.merge?.note?.from ?? ''}
				<span class="spacer"></span>
				<button class="btn" onclick={pending(abortMerge)}>Abort…</button>
			</div>
			{#if git.conflicts.length > 0}
				<SideSection title="Conflicts" count={git.conflicts.length}>
					{#each git.conflicts as path (path)}
						<SideItem label={displayPath(path)} mono onclick={() => tabs.open('conflicts')}>
							{#snippet trailing()}<span class="mark-text conflict">!</span>{/snippet}
						</SideItem>
					{/each}
				</SideSection>
			{/if}
		{/if}

		<SideSection title="Changes" count={git.changes.length} bind:open={open.changes}>
			{#if git.changes.length === 0}
				<div class="quiet-row small">Nothing to commit{git.branch ? ` on ${git.branch}` : ''}</div>
			{/if}
			{#if contentChanges.length > 0}
				<div class="sub">Content</div>
				{#each contentChanges as change (change.path)}
					{@const word = contentChange(change.kind)}
					<SideItem
						label={change.name}
						onclick={() => tabs.open('changes', '', { preview: true })}
						ondblclick={() => change.kind !== 'removed' && tabs.open('mod', change.path)}
						oncontextmenu={(e) => menus.at(e, contentChangeMenu(change))}
					>
						{#snippet lead()}
							<span class="twisty"></span>
							<input
								type="checkbox"
								checked={!git.excluded[change.path]}
								onclick={(e) => e.stopPropagation()}
								onchange={(e) => git.include([change.path], e.currentTarget.checked)}
							/>
						{/snippet}
						{#snippet trailing()}<span class="mark-text {word}">{changeMark(word)}</span>{/snippet}
					</SideItem>
				{/each}
			{/if}
			{#if fileChanges.length > 0}
				<div class="sub">Files</div>
				{#each tree as child (child.path)}
					{@render node(child, 0)}
				{/each}
			{/if}
		</SideSection>

		<SideSection
			title="Branches"
			count={git.locals.length + git.remoteOnly.length}
			bind:open={open.branches}
		>
			{#each git.locals as branch (branch.name)}
				{@const folder = git.worktreeFor(branch.name)}
				<SideItem
					label={branch.name}
					mono
					active={branch.current}
					onclick={() => tabs.open('branches', '', { preview: true })}
					oncontextmenu={(e) => menus.at(e, branchMenu(branch.name))}
				>
					{#snippet trailing()}
						{#if branch.current}<span class="small">this window</span
							>{:else if folder && git.windowFor(folder.path)}<span class="small">open</span>{/if}
					{/snippet}
				</SideItem>
			{/each}
			{#each git.remoteOnly as branch (branch.name)}
				<SideItem
					label={branch.name}
					mono
					onclick={() => tabs.open('branches', '', { preview: true })}
					oncontextmenu={(e) => menus.at(e, branchMenu(branch.name))}
				>
					{#snippet trailing()}<span class="small">remote</span>{/snippet}
				</SideItem>
			{/each}
		</SideSection>

		<SideSection title="Remotes" count={git.remotes.length} bind:open={open.remotes}>
			{#each git.remotes as remote (remote.name)}
				<SideItem label={remote.name} mono>
					{#snippet trailing()}<span class="small ellipsis url"
							>{remote.url.replace(/^https?:\/\/(www\.)?/, '').replace(/\.git$/, '')}</span
						>{/snippet}
				</SideItem>
			{:else}
				<div class="quiet-row small">No remote</div>
			{/each}
		</SideSection>

		<SideSection title="Tags" count={git.tags.length} bind:open={open.tags}>
			{#each git.tags.slice(0, 8) as tag (tag.name)}
				<SideItem
					label={tag.name}
					mono
					onclick={() => tabs.open('history', tag.commit)}
					oncontextmenu={(e) => menus.at(e, tagMenu(tag))}
				/>
			{/each}
		</SideSection>

		<SideSection title="Stashes" count={git.stashes.length} bind:open={open.stashes}>
			{#each git.stashes as stash (stash.reference)}
				<SideItem
					label={stash.message || stash.reference}
					oncontextmenu={(e) =>
						menus.at(e, [
							{ label: 'Apply and drop', run: () => void applyStash(stash.reference, true) },
							{ label: 'Apply', run: () => void applyStash(stash.reference, false) },
							'separator',
							{ label: 'Delete…', danger: true, run: () => void dropStash(stash.reference) },
						])}
				>
					{#snippet trailing()}<span class="small">{stash.relative}</span>{/snippet}
				</SideItem>
			{/each}
		</SideSection>
	</div>

	<div class="commit-box">
		<textarea
			class="message"
			rows="3"
			placeholder={git.merging ? (git.merge?.note?.message ?? 'Merge') : 'Message'}
			class:prefilled={git.merging}
			bind:value={git.message}
			onkeydown={onmessagekey}
			spellcheck="true"></textarea>
		{#if git.merging}
			<button
				class="btn primary commit"
				disabled={git.conflicts.length > 0}
				onclick={pending(commitMerge)}>Commit merge</button
			>
		{:else}
			<div class="actions">
				<label class="check small"><input type="checkbox" bind:checked={amend} />Amend</label>
				<span class="small">{included.length} of {total}</span>
				<span class="spacer"></span>
				<button
					class="btn primary"
					disabled={committing || !git.message.trim() || (included.length === 0 && !amend)}
					aria-busy={committing}
					onclick={submit}
				>
					Commit {included.length || ''}
					<span class="keys">{keysFor('git.commit') ?? ''}</span>
				</button>
			</div>
		{/if}
		{#if git.remoteName && (ahead > 0 || behind > 0)}
			<div class="sync">
				{#if behind > 0}
					<button
						class="btn"
						disabled={git.busy !== null}
						aria-busy={git.busy === 'pull'}
						onclick={() => void pull()}
					>
						<Icon icon={ArrowDownToLine} size={14} /> Pull {behind}
					</button>
				{/if}
				{#if ahead > 0}
					<button
						class="btn"
						disabled={git.busy !== null}
						aria-busy={git.busy === 'push'}
						onclick={() => void push()}
					>
						<Icon icon={ArrowUpFromLine} size={14} /> Push {ahead}
					</button>
				{/if}
			</div>
		{/if}
	</div>
{/if}

<style>
	.twisty {
		width: 12px;
		flex: none;
		display: inline-grid;
		place-items: center;
	}

	.sub {
		display: flex;
		align-items: center;
		min-height: 24px;
		margin-top: 2px;
		padding: 0 16px;
		background: var(--band);
		font-size: 12px;
		font-weight: 600;
	}

	.quiet-row {
		padding: 6px 16px;
	}

	.url {
		max-width: 60%;
	}

	.commit-box {
		flex: none;
		display: flex;
		flex-direction: column;
		border-top: 1px solid var(--line);
	}

	.message {
		display: block;
		width: 100%;
		padding: 10px 14px;
		background: var(--band);
		resize: none;
		font-size: 13px;
		line-height: 1.45;
	}

	.message:focus-visible {
		outline: none;
		box-shadow: inset 0 -2px 0 var(--focus);
	}

	.actions {
		display: flex;
		align-items: stretch;
		gap: 10px;
		min-height: 36px;
		padding-left: 14px;
		box-shadow: inset 0 1px 0 var(--line);
	}

	.actions > :is(label, span) {
		align-self: center;
	}

	.actions .btn,
	.commit {
		height: auto;
		min-height: 36px;
	}

	.sync {
		display: flex;
		gap: 1px;
		border-top: 1px solid var(--line);
	}

	.sync .btn {
		flex: 1;
		height: 32px;
	}
</style>

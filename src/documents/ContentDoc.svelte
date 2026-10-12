<script lang="ts">
	import { ArrowUp, Monitor, Pin, Search, Server, X } from '@lucide/svelte'
	import { tick, untrack } from 'svelte'

	import {
		buildRows,
		FILTER_LABELS,
		filterCounts,
		type ContentFilter,
		type ContentRow,
	} from '@/lib/content/rows'
	import {
		openAddContent,
		pasteList,
		removeContent,
		restoreRemoved,
		toggleDisabled,
		updateContent,
	} from '@/lib/flows/content'
	import { clickSelect, keySelect, type ListSelection } from '@/lib/listselect'
	import { contentMenu, localMenu } from '@/lib/menus'
	import { Table } from '@/lib/state/columns.svelte'
	import { content } from '@/lib/state/content.svelte'
	import { contentView } from '@/lib/state/contentview.svelte'
	import { menus } from '@/lib/state/menus.svelte'
	import { selection } from '@/lib/state/selection.svelte'
	import { tabs, type Doc } from '@/lib/state/tabs.svelte'
	import { ws } from '@/lib/state/workspace.svelte'
	import type { ProjectType } from '@/lib/tauri/types'
	import { typeName } from '@/lib/util/format'
	import Author from '@/ui/Author.svelte'
	import ColumnHead from '@/ui/ColumnHead.svelte'
	import Empty from '@/ui/Empty.svelte'
	import Icon from '@/ui/Icon.svelte'
	import ModIcon from '@/ui/ModIcon.svelte'
	import { pending } from '@/ui/pending'

	import DatapackSetup from './content/DatapackSetup.svelte'

	interface Props {
		doc: Doc
	}

	let { doc }: Props = $props()

	const type = $derived(doc.arg as ProjectType)
	let list = $state<ListSelection>({
		selected: untrack(() => selection.of(doc.id)),
		anchor: null,
		cursor: null,
	})
	const table = new Table(
		'content',
		[
			{ id: 'name', label: 'Name', min: 200 },
			{ id: 'version', label: 'Version', width: 140, min: 70, hide: 1 },
			{ id: 'side', label: 'Side', width: 56, min: 40, hide: 2 },
			{ id: 'source', label: 'Source', width: 110, min: 64, hide: 3 },
			{ id: 'usedBy', label: 'Used by', width: 160, min: 64, hide: 4 },
		],
		{ gap: 16, left: 20, right: 16 },
	)
	let searchInput = $state<HTMLInputElement | null>(null)
	let listElement = $state<HTMLDivElement | null>(null)

	const rows = $derived(
		buildRows(type, content.rowInputs, contentView.filter[type], contentView.query[type]),
	)
	const counts = $derived(filterCounts(type, content.rowInputs))
	const ids = $derived(rows.map((r) => r.id))
	const selectedMods = $derived(list.selected.filter((id) => content.find(id)))
	const selectedUpdates = $derived(selectedMods.filter((id) => content.updates[id]).length)
	const allDisabled = $derived(selectedMods.every((id) => content.find(id)?.disabled))
	const updates = $derived(rows.filter((r) => r.update).length)
	const total = $derived(content.ofType(type).length)
	const chips: { id: ContentFilter; label: string }[] = [
		{ id: 'all', label: 'All' },
		{ id: 'updates', label: 'Updates' },
		{ id: 'uncommitted', label: 'Uncommitted' },
	]

	function select(next: ListSelection): void {
		list = next
		selection.set(doc.id, next.selected)
	}

	function onrowclick(row: ContentRow, event: MouseEvent): void {
		select(clickSelect(list, ids, row.id, event))
	}

	function open(row: ContentRow, side = false): void {
		if (row.mod) tabs.open('mod', row.mod.key, { side })
	}

	async function reveal(id: string | null): Promise<void> {
		if (!id) return
		await tick()
		listElement
			?.querySelector(`[data-id="${CSS.escape(id)}"]`)
			?.scrollIntoView({ block: 'nearest' })
	}

	function onkeydown(event: KeyboardEvent): void {
		const next = keySelect(list, ids, event)
		if (next) {
			event.preventDefault()
			select(next)
			void reveal(next.cursor)
			return
		}
		const row = rows.find((r) => r.id === list.cursor)
		if (event.key === 'Enter' && row) {
			event.preventDefault()
			open(row, event.metaKey || event.ctrlKey)
		}
	}

	function onfind(event: KeyboardEvent): void {
		if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === 'f') {
			event.preventDefault()
			searchInput?.focus()
		}
	}

	function rowAction(event: MouseEvent, action: () => Promise<void>): void {
		event.stopPropagation()
		pending(action)(event)
	}

	function clearFilter(): void {
		contentView.query[type] = ''
		contentView.show(type, 'all')
	}

	function oncontextmenu(row: ContentRow, event: MouseEvent): void {
		if (!list.selected.includes(row.id)) {
			select({ selected: [row.id], anchor: row.id, cursor: row.id })
		}
		if (row.removed) {
			menus.at(event, [{ label: 'Restore', run: () => void restoreRemoved(row.removed!.path) }])
			return
		}
		if (row.local) {
			menus.at(event, localMenu(row.local))
			return
		}
		menus.at(event, contentMenu(list.selected.filter((id) => content.find(id))))
	}
</script>

<svelte:window onkeydown={onfind} />

{#if type === 'datapack' && !ws.manifest?.datapacks}
	<DatapackSetup />
{:else}
	{#if list.selected.length > 1}
		<div class="bar doc-toolbar">
			<span class="doc-title">{list.selected.length} selected</span>
			<span class="spacer"></span>
			{#if selectedUpdates > 0}
				<button class="btn" onclick={pending(() => updateContent(selectedMods))}
					>Update {selectedUpdates}</button
				>
			{/if}
			<button class="btn" onclick={pending(() => toggleDisabled(selectedMods))}
				>{allDisabled ? 'Enable' : 'Disable'}</button
			>
			<button class="btn danger" onclick={pending(() => removeContent(selectedMods))}>Remove</button
			>
			<button class="btn" onclick={() => select({ selected: [], anchor: null, cursor: null })}
				>Clear <span class="keys">esc</span></button
			>
		</div>
	{:else}
		<div class="bar doc-toolbar">
			<span class="doc-title">{typeName(type, true)}</span>
			<span class="count-text">{total}</span>
			{#each chips as chip (chip.id)}
				<button
					class="chip"
					class:on={contentView.filter[type] === chip.id}
					onclick={() => contentView.show(type, chip.id)}
				>
					{chip.label}
					{#if chip.id !== 'all' && counts[chip.id] > 0}<span class="count-text"
							>{counts[chip.id]}</span
						>{/if}
				</button>
			{/each}
			{#if contentView.filter[type] !== 'all' && !chips.some((c) => c.id === contentView.filter[type])}
				<button class="chip on" onclick={() => contentView.show(type, 'all')}>
					{FILTER_LABELS[contentView.filter[type]]}
					<Icon icon={X} size={12} />
				</button>
			{/if}
			<label class="search">
				<Icon icon={Search} size={14} />
				<input
					bind:this={searchInput}
					bind:value={contentView.query[type]}
					placeholder="Filter"
					spellcheck="false"
				/>
			</label>
			<span class="spacer"></span>
			{#if updates > 0}
				<button
					class="btn"
					onclick={pending(() => updateContent(rows.filter((r) => r.update).map((r) => r.id)))}
					>Update {updates}</button
				>
			{/if}
			<button class="btn primary" onclick={() => openAddContent(type)}
				>Add {typeName(type, true).toLowerCase()}</button
			>
		</div>
	{/if}

	{#if total === 0 && rows.length === 0}
		<Empty title="Add your first {typeName(type, true).toLowerCase()}">
			<button class="btn primary" onclick={() => openAddContent(type)}>Browse</button>
			<button class="btn" onclick={pending(() => pasteList())}>Paste a list…</button>
		</Empty>
	{:else}
		<div class="scroll">
			<ColumnHead {table} />
			<div
				bind:this={listElement}
				class="list"
				role="listbox"
				aria-multiselectable="true"
				tabindex="0"
				{onkeydown}
			>
				{#each rows as row (row.id)}
					<div
						class="row"
						style={table.style}
						data-id={row.id}
						class:selected={list.selected.includes(row.id)}
						class:cursor={list.cursor === row.id && list.selected.length > 1}
						role="option"
						aria-selected={list.selected.includes(row.id)}
						tabindex="-1"
						onclick={(e) => onrowclick(row, e)}
						ondblclick={() => open(row)}
						oncontextmenu={(e) => oncontextmenu(row, e)}
					>
						{#if row.change}<span class="mark {row.change}"></span>{/if}
						<span class="cell-name">
							<ModIcon url={row.icon} size={32} />
							<span class="lines">
								<span class="line">
									<span class="name">{row.name}</span>
									{#if row.words.length > 0 || row.problem}
										<span class="words"
											>({#each row.words as word, i (word)}{#if i > 0}{', '}{/if}<span
													class:removed={word === 'removed'}>{word}</span
												>{/each}{#if row.problem}{#if row.words.length > 0}{', '}{/if}<span
													class="problem">problem</span
												>{/if})</span
										>
									{/if}
								</span>
								<span class="line sub">
									{#if row.author}
										<Author name={row.author} icon={row.authorIcon} />
									{:else if row.local}
										<span class="mono ellipsis">{row.local.filename}</span>
									{:else if row.removed}
										<span>removed from the pack</span>
									{/if}
								</span>
							</span>
						</span>
						{#if table.has('version')}
							<span class="lines">
								<span class="text mono" title={row.mod?.pinned ? 'Pinned' : row.version}
									>{#if row.mod?.pinned}<Icon icon={Pin} size={12} />{/if}{row.version || '—'}</span
								>
								{#if row.update}
									<span class="text mono sub update" title={row.update.versionNumber}
										><Icon icon={ArrowUp} size={12} />{row.update.versionNumber}</span
									>
								{/if}
							</span>
						{/if}
						{#if table.has('side')}
							<span class="side" title={row.side?.label}>
								{#if row.side?.client}<Icon icon={Monitor} size={14} />{/if}
								{#if row.side?.server}<Icon icon={Server} size={14} />{/if}
								{#if !row.side?.client && !row.side?.server}<span class="hint">—</span>{/if}
							</span>
						{/if}
						{#if table.has('source')}
							<span class="ellipsis" title={[row.source, ...row.otherSources].join(', ')}
								>{row.source}{#if row.otherSources.length > 0}<span class="hint"
										>{` +${row.otherSources.length}`}</span
									>{/if}</span
							>
						{/if}
						{#if table.has('usedBy')}
							<span class="ellipsis" title={row.usedBy.join(', ')}
								>{row.usedBy.length ? row.usedBy.join(', ') : '—'}</span
							>
						{/if}
						{#if row.removed || row.update}
							<span class="actions">
								{#if row.removed}
									<button
										class="btn"
										onclick={(e) => rowAction(e, () => restoreRemoved(row.removed!.path))}
										>Restore</button
									>
								{:else if row.update}
									<button class="btn" onclick={(e) => rowAction(e, () => updateContent([row.id]))}
										>Update</button
									>
								{/if}
							</span>
						{/if}
					</div>
				{:else}
					<Empty title="Nothing matches">
						<button class="btn" onclick={clearFilter}>Clear filter</button>
					</Empty>
				{/each}
			</div>
		</div>
	{/if}
{/if}

<style>
	.search {
		display: flex;
		align-items: center;
		gap: 6px;
		align-self: stretch;
		margin-left: 6px;
		padding: 0 10px;
		background: var(--band);
		width: 180px;
	}

	.search input {
		flex: 1;
		min-width: 0;
		font-size: 12.5px;
	}

	.list:focus {
		outline: none;
	}

	.row {
		position: relative;
		align-items: center;
		height: 48px;
		scroll-margin-top: var(--row);
		border-bottom: 1px solid var(--line);
		font-size: 13.5px;
		white-space: nowrap;
	}

	.row:hover {
		background: var(--hover);
	}

	.row.selected {
		background: var(--selected-idle);
	}

	.list:focus-within .row.selected {
		background: var(--selected);
	}

	.row.cursor {
		outline: 2px solid var(--focus);
		outline-offset: -2px;
	}

	.cell-name {
		display: flex;
		align-items: center;
		gap: 12px;
		min-width: 0;
	}

	.lines {
		display: flex;
		flex-direction: column;
		gap: 2px;
		min-width: 0;
	}

	.line {
		display: flex;
		align-items: center;
		gap: 8px;
		min-width: 0;
	}

	.name {
		flex: 0 1 auto;
		min-width: 48px;
		overflow: hidden;
		text-overflow: ellipsis;
		font-weight: 500;
	}

	.words {
		flex: 0 100 auto;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		color: var(--grey);
		font-size: 12px;
	}

	.words .removed {
		color: var(--red);
	}

	.words .problem {
		color: var(--yellow);
	}

	.side {
		display: flex;
		align-items: center;
		gap: 4px;
		min-width: 0;
		overflow: hidden;
	}

	.sub {
		font-size: 12px;
		gap: 6px;
	}

	.text {
		display: block;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.update {
		color: var(--blue);
	}

	.text :global(svg) {
		display: inline-block;
		margin-right: 3px;
		vertical-align: -2px;
	}

	.actions {
		position: absolute;
		top: 0;
		right: 0;
		bottom: 0;
		display: none;
		align-items: stretch;
		background: var(--doc);
	}

	.row:hover .actions,
	.row.selected .actions {
		display: flex;
	}

	.actions .btn {
		height: auto;
	}
</style>

<script lang="ts">
	import { ArrowUp, Pin, Search, X } from '@lucide/svelte'
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
	import { content } from '@/lib/state/content.svelte'
	import { contentView } from '@/lib/state/contentview.svelte'
	import { menus } from '@/lib/state/menus.svelte'
	import { selection } from '@/lib/state/selection.svelte'
	import { tabs, type Doc } from '@/lib/state/tabs.svelte'
	import { ws } from '@/lib/state/workspace.svelte'
	import type { ProjectType } from '@/lib/tauri/types'
	import { typeName } from '@/lib/util/format'
	import Author from '@/ui/Author.svelte'
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
		<div class="grid head">
			<span>Name</span>
			<span>Version</span>
			<span>Side</span>
			<span>Source</span>
			<span>Used by</span>
		</div>
		<div
			bind:this={listElement}
			class="scroll list"
			role="listbox"
			aria-multiselectable="true"
			tabindex="0"
			{onkeydown}
		>
			{#each rows as row (row.id)}
				<div
					class="grid row"
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
								{#each row.words as word (word)}
									<span class="tag" class:removed={word === 'removed'}>{word}</span>
								{/each}
								{#if row.problem}<span class="tag changed">problem</span>{/if}
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
					<span class="ellipsis">{row.side}</span>
					<span class="ellipsis"
						>{row.source}{#each row.otherSources as other (other)}<span class="hint">
								· {other}</span
							>{/each}</span
					>
					<span class="ellipsis">{row.usedBy.length ? row.usedBy.join(', ') : '—'}</span>
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

	.grid {
		display: grid;
		grid-template-columns: minmax(0, 2fr) minmax(150px, 1fr) 112px 112px minmax(0, 180px);
		column-gap: 16px;
		align-items: center;
		padding: 0 16px 0 20px;
	}

	.head {
		flex: none;
		height: var(--row);
		background: var(--band);
		border-bottom: 1px solid var(--line);
		font-size: 12.5px;
		font-weight: 600;
	}

	.row {
		position: relative;
		height: 48px;
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
		overflow: hidden;
		text-overflow: ellipsis;
		font-weight: 500;
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

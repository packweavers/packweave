<script lang="ts">
	import { Columns2, Ellipsis, X } from '@lucide/svelte'
	import { onMount, tick } from 'svelte'

	import { docIcon, docNeedsYou, docTitle } from '@/documents/registry'
	import { closeDoc, dropOutside, moveToPanel } from '@/lib/flows/tabs'
	import { tabMenu } from '@/lib/menus'
	import { files } from '@/lib/state/files.svelte'
	import { menus } from '@/lib/state/menus.svelte'
	import { tabDrag } from '@/lib/state/tabdrag.svelte'
	import { tabs, type Doc, type Group } from '@/lib/state/tabs.svelte'
	import { ws } from '@/lib/state/workspace.svelte'
	import Icon from '@/ui/Icon.svelte'
	import IconButton from '@/ui/IconButton.svelte'

	interface Props {
		group: Group
		index: number
		focused: boolean
	}

	interface Press {
		doc: Doc
		x: number
		y: number
		pointer: number
	}

	let { group, index, focused }: Props = $props()

	const DRAG_DISTANCE = 5

	let scroller = $state<HTMLDivElement | null>(null)
	let thumb = $state({ left: 0, width: 0, visible: false })
	let press: Press | null = null
	let swallowClick = false
	let thumbDrag: { x: number; scroll: number } | null = null

	const spot = $derived(tabDrag.spot?.group === index ? tabDrag.spot : null)

	onMount(() => {
		if (!scroller) return
		const observer = new ResizeObserver(measure)
		observer.observe(scroller)
		return () => observer.disconnect()
	})

	$effect(() => {
		const active = group.active
		void group.docs.length
		void tick().then(() => {
			measure()
			if (!active || !scroller) return
			scroller
				.querySelector<HTMLElement>(`[data-tab="${CSS.escape(active)}"]`)
				?.scrollIntoView({ block: 'nearest', inline: 'nearest' })
		})
	})

	function measure(): void {
		if (!scroller) return
		const { scrollLeft, scrollWidth, clientWidth } = scroller
		const visible = scrollWidth > clientWidth + 1
		thumb = {
			left: visible ? (scrollLeft / scrollWidth) * clientWidth : 0,
			width: visible ? (clientWidth / scrollWidth) * clientWidth : 0,
			visible,
		}
	}

	function onwheel(event: WheelEvent): void {
		if (!scroller || Math.abs(event.deltaX) >= Math.abs(event.deltaY)) return
		scroller.scrollLeft += event.deltaY
		event.preventDefault()
	}

	function grabThumb(event: PointerEvent): void {
		if (!scroller) return
		event.preventDefault()
		thumbDrag = { x: event.clientX, scroll: scroller.scrollLeft }
		const handle = event.currentTarget as HTMLElement
		handle.setPointerCapture(event.pointerId)
	}

	function moveThumb(event: PointerEvent): void {
		if (!thumbDrag || !scroller) return
		const ratio = scroller.scrollWidth / scroller.clientWidth
		scroller.scrollLeft = thumbDrag.scroll + (event.clientX - thumbDrag.x) * ratio
	}

	function listAll(event: MouseEvent): void {
		menus.below(
			event.currentTarget as Element,
			tabs.all.map((doc) => ({ label: docTitle(doc), run: () => tabs.activate(doc.id) })),
		)
	}

	function onpointerdown(event: PointerEvent, doc: Doc): void {
		swallowClick = false
		if (event.button !== 0 || (event.target as Element).closest('.close')) return
		press = { doc, x: event.clientX, y: event.clientY, pointer: event.pointerId }
		const tab = event.currentTarget as HTMLElement
		tab.setPointerCapture(event.pointerId)
	}

	function onpointermove(event: PointerEvent): void {
		if (!press || event.pointerId !== press.pointer) return
		if (!tabDrag.doc) {
			const distance = Math.hypot(event.clientX - press.x, event.clientY - press.y)
			if (distance < DRAG_DISTANCE) return
			tabDrag.begin(press.doc)
		}
		tabDrag.move(event.clientX, event.clientY)
	}

	function onpointerup(event: PointerEvent): void {
		if (!press || event.pointerId !== press.pointer) return
		const doc = press.doc
		press = null
		if (!tabDrag.doc) return
		swallowClick = true
		const target = tabDrag.spot
		const inside = tabDrag.inside
		const panel = tabDrag.panel
		tabDrag.end()
		if (panel) {
			moveToPanel(doc)
		} else if (target) {
			tabs.moveTo(doc.id, target.group, target.index)
		} else if (!(inside && ws.detached && tabs.all.length === 1)) {
			void dropOutside(doc, event.screenX, event.screenY)
		}
	}

	function cancel(): void {
		press = null
		tabDrag.end()
	}

	function onclick(doc: Doc): void {
		if (swallowClick) {
			swallowClick = false
			return
		}
		tabs.activate(doc.id)
	}

	function onkeydown(event: KeyboardEvent): void {
		if (event.key === 'Escape' && tabDrag.doc) {
			event.preventDefault()
			cancel()
		}
	}
</script>

<svelte:window {onkeydown} />

<div class="tabs" role="tablist" tabindex="-1" data-strip={index}>
	<div class="scroller" id="tabs-{index}" bind:this={scroller} onscroll={measure} {onwheel}>
		{#each group.docs as doc, at (doc.id)}
			{@const active = doc.id === group.active}
			{@const dirty = doc.kind === 'file' && files.isDirty(doc.arg)}
			<div
				class="tab"
				class:active
				class:idle={active && !focused}
				class:preview={doc.preview}
				class:pinned={doc.pinned}
				class:alert={docNeedsYou(doc)}
				class:lifted={tabDrag.doc?.id === doc.id}
				class:drop-before={spot?.index === at}
				data-tab={doc.id}
				role="tab"
				tabindex="0"
				aria-selected={active}
				onpointerdown={(e) => onpointerdown(e, doc)}
				{onpointermove}
				{onpointerup}
				onpointercancel={cancel}
				onclick={() => onclick(doc)}
				ondblclick={() => tabs.keep(doc.id)}
				onauxclick={(e) => e.button === 1 && void closeDoc(doc)}
				oncontextmenu={(e) => menus.at(e, tabMenu(doc))}
				onkeydown={(e) => e.key === 'Enter' && tabs.activate(doc.id)}
			>
				<Icon icon={docIcon(doc)} size={14} />
				<span class="name">{docTitle(doc)}</span>
				{#if dirty}
					<span class="dirty" aria-label="Unsaved"></span>
				{:else if !doc.pinned}
					<button
						class="close"
						aria-label="Close"
						onclick={(e) => {
							e.stopPropagation()
							void closeDoc(doc)
						}}
					>
						<Icon icon={X} size={12} />
					</button>
				{/if}
			</div>
		{/each}
		<div class="end" class:drop-before={spot?.index === group.docs.length}></div>
	</div>
	{#if thumb.visible}
		<div
			class="thumb"
			role="scrollbar"
			aria-controls="tabs-{index}"
			aria-valuenow={Math.round(thumb.left)}
			tabindex="-1"
			style:left="{thumb.left}px"
			style:width="{thumb.width}px"
			onpointerdown={grabThumb}
			onpointermove={moveThumb}
			onpointerup={() => (thumbDrag = null)}
			onpointercancel={() => (thumbDrag = null)}
		></div>
	{/if}
	<div class="actions">
		<IconButton
			icon={Columns2}
			label="Split right"
			command="editor.split"
			onclick={() => tabs.split()}
		/>
		<IconButton icon={Ellipsis} label="All tabs" onclick={listAll} />
	</div>
</div>

<style>
	.tabs {
		position: relative;
		flex: none;
		height: var(--bar);
		display: flex;
		align-items: stretch;
		background: var(--chrome);
		border-bottom: 1px solid var(--line);
	}

	.scroller {
		flex: 1;
		min-width: 0;
		display: flex;
		align-items: stretch;
		overflow-x: auto;
		overflow-y: hidden;
		scrollbar-width: none;
	}

	.scroller::-webkit-scrollbar {
		display: none;
	}

	.thumb {
		position: absolute;
		bottom: 0;
		height: 3px;
		background: var(--fill);
		opacity: 0;
		transition: opacity 120ms;
	}

	.tabs:hover .thumb {
		opacity: 1;
	}

	.thumb:hover,
	.thumb:active {
		height: 5px;
		background: var(--selected);
	}

	.tab {
		position: relative;
		display: flex;
		align-items: center;
		gap: 6px;
		min-width: 0;
		max-width: 220px;
		padding: 0 6px 0 12px;
		border-right: 1px solid var(--line);
		font-size: 12.5px;
		white-space: nowrap;
		flex: none;
		touch-action: none;
	}

	.tab:hover:not(.active) {
		background: var(--hover);
	}

	.tab.active {
		background: var(--doc);
		margin-bottom: -1px;
		box-shadow: inset 0 2px 0 var(--text);
		font-weight: 500;
	}

	.tab.idle {
		box-shadow: inset 0 2px 0 var(--grey);
	}

	.tab.lifted {
		opacity: 0.4;
	}

	.tab.preview .name {
		font-style: italic;
	}

	.tab.alert .name {
		color: var(--red);
	}

	.tab.pinned {
		padding-right: 12px;
	}

	.drop-before::before {
		content: '';
		position: absolute;
		left: -1px;
		top: 4px;
		bottom: 4px;
		width: 2px;
		background: var(--focus);
	}

	.end {
		position: relative;
		flex: none;
		width: 8px;
	}

	.name {
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.close {
		width: 18px;
		height: 18px;
		display: inline-grid;
		place-items: center;
		flex: none;
		visibility: hidden;
	}

	.tab:hover .close,
	.tab.active .close {
		visibility: visible;
	}

	.close:hover {
		background: var(--fill);
	}

	.dirty {
		width: 8px;
		height: 8px;
		margin: 0 5px;
		border-radius: 50%;
		background: var(--text);
		flex: none;
	}

	.actions {
		display: flex;
		align-items: stretch;
		flex: none;
		border-left: 1px solid var(--line);
	}

	.actions :global(.icon-btn) {
		width: var(--bar);
		height: auto;
	}
</style>

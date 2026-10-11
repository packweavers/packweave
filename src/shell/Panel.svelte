<script lang="ts">
	import { X } from '@lucide/svelte'

	import { dropPanelOutside } from '@/lib/flows/tabs'
	import { panelMenu } from '@/lib/menus'
	import { content } from '@/lib/state/content.svelte'
	import { layout, type PanelTab } from '@/lib/state/layout.svelte'
	import { menus } from '@/lib/state/menus.svelte'
	import { panels } from '@/lib/state/panels.svelte'
	import { tabDrag } from '@/lib/state/tabdrag.svelte'
	import { bareDoc, tabs } from '@/lib/state/tabs.svelte'
	import { tasks } from '@/lib/state/tasks.svelte'
	import GameLog from '@/panel/GameLog.svelte'
	import Output from '@/panel/Output.svelte'
	import Problems from '@/panel/Problems.svelte'
	import IconButton from '@/ui/IconButton.svelte'
	import Splitter from '@/ui/Splitter.svelte'

	interface Press {
		tab: PanelTab
		x: number
		y: number
		pointer: number
	}

	const DRAG_DISTANCE = 5

	let press: Press | null = null
	let swallowClick = false

	const tabsList = $derived(
		(
			[
				{ id: 'problems', label: 'Problems', count: content.problems.length },
				{ id: 'output', label: 'Output', count: tasks.failures },
				{ id: 'gamelog', label: 'Game log', count: 0 },
			] satisfies { id: PanelTab; label: string; count: number }[]
		).filter((tab) => panels.shown.includes(tab.id)),
	)
	const current = $derived(
		tabsList.some((tab) => tab.id === layout.panelTab)
			? layout.panelTab
			: (tabsList[0]?.id ?? null),
	)

	function onpointerdown(event: PointerEvent, tab: PanelTab): void {
		swallowClick = false
		if (event.button !== 0) return
		press = { tab, x: event.clientX, y: event.clientY, pointer: event.pointerId }
		;(event.currentTarget as HTMLElement).setPointerCapture(event.pointerId)
	}

	function onpointermove(event: PointerEvent): void {
		if (!press || event.pointerId !== press.pointer) return
		if (!tabDrag.doc) {
			if (Math.hypot(event.clientX - press.x, event.clientY - press.y) < DRAG_DISTANCE) return
			tabDrag.begin(bareDoc(press.tab))
		}
		tabDrag.move(event.clientX, event.clientY)
	}

	function onpointerup(event: PointerEvent): void {
		if (!press || event.pointerId !== press.pointer) return
		const tab = press.tab
		press = null
		if (!tabDrag.doc) return
		swallowClick = true
		const target = tabDrag.spot
		const home = tabDrag.panel
		tabDrag.end()
		if (home) return
		if (target) {
			tabs.open(tab)
			tabs.moveTo(tab, target.group, target.index)
		} else {
			void dropPanelOutside(tab, event.screenX, event.screenY)
		}
	}

	function cancel(): void {
		press = null
		tabDrag.end()
	}

	function onclick(tab: PanelTab): void {
		if (swallowClick) {
			swallowClick = false
			return
		}
		layout.openPanel(tab)
	}
</script>

<section class="panel" style:height="{layout.panelHeight}px" aria-label="Panel">
	<div class="edge">
		<Splitter
			axis="y"
			invert
			bind:value={layout.panelHeight}
			min={120}
			max={600}
			ondone={() => layout.persist()}
		/>
	</div>
	<div class="panel-tabs" role="tablist" data-panel-bar>
		{#each tabsList as tab (tab.id)}
			<button
				class="panel-tab"
				class:on={current === tab.id}
				class:lifted={tabDrag.doc?.id === tab.id}
				role="tab"
				aria-selected={current === tab.id}
				onpointerdown={(e) => onpointerdown(e, tab.id)}
				{onpointermove}
				{onpointerup}
				onpointercancel={cancel}
				onclick={() => onclick(tab.id)}
				oncontextmenu={(e) => menus.at(e, panelMenu(tab.id))}
			>
				{tab.label}
				{#if tab.count > 0}<span class="count-text">{tab.count}</span>{/if}
			</button>
		{/each}
		<span class="spacer"></span>
		<IconButton
			icon={X}
			label="Close panel"
			command="layout.panel"
			onclick={() => layout.togglePanel()}
		/>
	</div>
	<div class="panel-body">
		{#if current === 'problems'}
			<Problems />
		{:else if current === 'output'}
			<Output />
		{:else if current === 'gamelog'}
			<GameLog />
		{/if}
	</div>
</section>

<style>
	.panel {
		position: relative;
		flex: none;
		display: flex;
		flex-direction: column;
		border-top: 1px solid var(--line);
		background: var(--doc);
	}

	.edge {
		position: absolute;
		top: 0;
		left: 0;
		right: 0;
	}

	.panel-tabs {
		flex: none;
		height: var(--bar);
		display: flex;
		align-items: stretch;
		background: var(--chrome);
		border-bottom: 1px solid var(--line);
	}

	.panel-tabs :global(.icon-btn) {
		width: var(--bar);
		height: auto;
	}

	.panel-tab {
		display: inline-flex;
		align-items: center;
		gap: 6px;
		padding: 0 12px;
		font-size: 12px;
		font-weight: 500;
	}

	.panel-tab.on {
		font-weight: 600;
		background: var(--selected);
		box-shadow: inset 0 -2px 0 var(--text);
	}

	.panel-tab.lifted {
		opacity: 0.5;
	}

	.panel-body {
		flex: 1;
		min-height: 0;
		display: flex;
		flex-direction: column;
		overflow: hidden;
	}
</style>

<script lang="ts">
	import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow'
	import { getCurrentWindow } from '@tauri-apps/api/window'
	import { onMount, untrack } from 'svelte'

	import { dropFiles } from '@/lib/flows/content'
	import { confirmUnsaved, importDropped } from '@/lib/flows/pack'
	import { receiveTab } from '@/lib/flows/tabs'
	import { app } from '@/lib/state/app.svelte'
	import { handleKey } from '@/lib/state/commands.svelte'
	import { content } from '@/lib/state/content.svelte'
	import { dialogs } from '@/lib/state/dialogs.svelte'
	import { files } from '@/lib/state/files.svelte'
	import { git } from '@/lib/state/git.svelte'
	import { instance } from '@/lib/state/instance.svelte'
	import { panels } from '@/lib/state/panels.svelte'
	import { tabs } from '@/lib/state/tabs.svelte'
	import { toasts } from '@/lib/state/toasts.svelte'
	import { windowState } from '@/lib/state/window.svelte'
	import { ws } from '@/lib/state/workspace.svelte'
	import type { IncomingTab } from '@/lib/tauri/types'
	import { windowPack } from '@/lib/tauri/windows'
	import { isMac } from '@/lib/util/platform'
	import ResizeEdges from '@/shell/ResizeEdges.svelte'
	import TabWindow from '@/shell/TabWindow.svelte'
	import TitleBar from '@/shell/TitleBar.svelte'
	import Workbench from '@/shell/Workbench.svelte'
	import DialogHost from '@/ui/DialogHost.svelte'
	import MenuHost from '@/ui/MenuHost.svelte'
	import Toasts from '@/ui/Toasts.svelte'
	import Welcome from '@/welcome/Welcome.svelte'

	const win = getCurrentWindow()
	let dragging = $state(false)
	let statusKey = ''

	onMount(() => {
		if (!isMac) void win.show()
		void windowPack()
			.then((pack) => {
				if (pack) void ws.open(pack.dir, { detached: pack.detached, tab: pack.tab })
			})
			.catch((error) => toasts.error(error))
		if (win.label === 'main') void app.checkForUpdate()

		const stopFollowing = windowState.follow()
		const stopSyncing = ws.follow()
		const stopPanels = panels.follow()
		const view = getCurrentWebviewWindow()
		const stops = [
			view.listen<IncomingTab>('receive-tab', ({ payload }) =>
				receiveTab(payload.tab, payload.x, payload.y),
			),
			view.listen<string>('show-doc', ({ payload }) => tabs.activate(payload)),
			win.onFocusChanged(({ payload: focused }) => {
				if (!focused || !ws.isOpen) return
				void git.refresh()
				void files.refresh()
				instance.scanIfStale(20_000)
				void content.recheckDropped()
			}),
			win.onCloseRequested(async (event) => {
				if (files.dirty.length === 0) return
				event.preventDefault()
				if (await confirmUnsaved()) await win.destroy()
			}),
			view.onDragDropEvent(({ payload }) => {
				if (payload.type === 'enter' || payload.type === 'over') {
					dragging = true
				} else if (payload.type === 'leave') {
					dragging = false
				} else {
					dragging = false
					onDrop(payload.paths)
				}
			}),
			view.listen('close-pack', () => void ws.close()),
		]
		const online = () => void content.recheckDropped(true)
		window.addEventListener('online', online)
		return () => {
			stopFollowing()
			stopSyncing()
			stopPanels()
			window.removeEventListener('online', online)
			for (const stop of stops) void stop.then((unlisten) => unlisten())
		}
	})

	$effect(() => {
		if (ws.detached && ws.isOpen && tabs.all.length === 0) void win.close()
	})

	$effect(() => {
		panels.announce()
	})

	$effect(() => {
		const status = git.status
		if (!status?.isRepo) return
		const key = [status.head, ...status.changes.map((c) => `${c.status} ${c.path}`)].join('\n')
		if (key === statusKey) return
		statusKey = key
		untrack(() => void content.refreshChanges())
	})

	function onDrop(paths: string[]): void {
		if (ws.isOpen) {
			const usable = paths.filter((p) => /\.(jar|zip)$/i.test(p))
			if (usable.length > 0) void dropFiles(usable)
			return
		}
		void importDropped(paths)
	}

	function onkeydown(event: KeyboardEvent): void {
		if (event.defaultPrevented || dialogs.current) return
		handleKey(event)
	}
</script>

<svelte:window {onkeydown} />

<div class="window">
	{#if ws.isOpen && ws.detached}
		<TabWindow />
	{:else}
		<TitleBar />
		{#if ws.isOpen}
			<Workbench />
		{:else if ws.opening}
			<div class="opening"><span class="spinner"></span></div>
		{:else}
			<Welcome />
		{/if}
	{/if}
</div>

<MenuHost />
<DialogHost />
<Toasts />
{#if !isMac}
	<ResizeEdges />
{/if}
{#if dragging}
	<div class="drop">{ws.isOpen ? 'Drop to add to the pack' : 'Drop to import'}</div>
{/if}

<style>
	.window {
		height: 100%;
		display: flex;
		flex-direction: column;
	}

	.opening {
		flex: 1;
		display: grid;
		place-items: center;
	}

	.drop {
		position: fixed;
		inset: 0;
		z-index: 80;
		display: grid;
		place-items: center;
		background: var(--scrim);
		color: var(--text);
		font-size: 15px;
		font-weight: 600;
		pointer-events: none;
	}
</style>

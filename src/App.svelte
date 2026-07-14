<script lang="ts">
	import { onMount } from 'svelte'
	import { getCurrentWindow } from '@tauri-apps/api/window'
	import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow'
	import Toolbar from './components/Toolbar.svelte'
	import WindowControls from './components/WindowControls.svelte'
	import WelcomeView from './components/WelcomeView.svelte'
	import { isMac } from './util'
	import ContentView from './components/ContentView.svelte'
	import FilesView from './components/FilesView.svelte'
	import InstanceView from './components/InstanceView.svelte'
	import SourceView from './components/SourceView.svelte'
	import NewPackModal from './components/NewPackModal.svelte'
	import StartPackModal from './components/StartPackModal.svelte'
	import IntroModal from './components/IntroModal.svelte'
	import SettingsModal from './components/SettingsModal.svelte'
	import ExportModal from './components/ExportModal.svelte'
	import InstancePicker from './components/InstancePicker.svelte'
	import AddPalette from './components/AddPalette.svelte'
	import SearchPanel from './components/SearchPanel.svelte'
	import CloneModal from './components/CloneModal.svelte'
	import AuthModal from './components/AuthModal.svelte'
	import DeleteConfirm from './components/content/DeleteConfirm.svelte'
	import ToastHost from './components/ToastHost.svelte'
	import CommandPalette from './components/CommandPalette.svelte'
	import DropModal from './components/DropModal.svelte'
	import ContextMenu from './components/ui/ContextMenu.svelte'
	import { api } from './api'
	import { store } from './lib/store.svelte'
	import { fileFind } from './lib/filefind.svelte'
	import type { DetectedInstance, DroppedFile } from './types'

	let showStart = $state(false)
	let showNew = $state(false)
	let showClone = $state(false)
	let showExport = $state(false)
	let showLink = $state(false)
	let showPickForNew = $state(false)
	let newInstance = $state<DetectedInstance | null>(null)
	let showAdd = $state(false)
	let showFind = $state(false)
	let showPalette = $state(false)
	let dragging = $state(false)
	let dropFiles = $state<DroppedFile[] | null>(null)
	let fileToOpen = $state<string | undefined>(undefined)
	let filesKey = $state(0)

	const showIntro = $derived(
		!store.hasPack && !store.getPref('introSeen', false) && store.recents.length === 0,
	)

	$effect(() => {
		if (store.hasPack) {
			showStart = false
			showNew = false
			showClone = false
		}
	})

	function onStartPick(choice: 'instance' | 'scratch' | 'import' | 'clone' | 'open') {
		if (choice === 'scratch') {
			newInstance = null
			showNew = true
		} else if (choice === 'instance') {
			showPickForNew = true
		} else if (choice === 'import') {
			void store.importPack()
		} else if (choice === 'clone') {
			showClone = true
		} else if (choice === 'open') {
			void store.openPack()
		}
	}

	function onPickForNew(inst: DetectedInstance) {
		showPickForNew = false
		newInstance = inst
		showNew = true
	}

	onMount(() => {
		store.initUi()
		if (!isMac) getCurrentWindow().setDecorations(false).catch(() => {})
		void store.checkForUpdate()
		const unlisten = getCurrentWebviewWindow().onDragDropEvent((event) => {
			const t = event.payload.type
			if (t === 'enter' || t === 'over') dragging = true
			else if (t === 'leave') dragging = false
			else if (t === 'drop') {
				dragging = false
				void handleDrop(event.payload.paths)
			}
		})
		return () => {
			void unlisten.then((u) => u())
		}
	})

	async function handleDrop(paths: string[]) {
		const ext = (p: string) => p.split('.').pop()?.toLowerCase() ?? ''
		if (!store.hasPack) {
			const pack = paths.find((p) => ['mrpack', 'zip'].includes(ext(p)))
			if (pack) await store.importPackFrom(pack)
			return
		}
		const jars = paths.filter((p) => ['jar', 'zip'].includes(ext(p)))
		if (!jars.length) {
			if (paths.some((p) => ext(p) === 'mrpack')) {
				store.notify('info', 'Close this pack first to import an .mrpack.')
			} else {
				store.notify('info', 'Drop .jar or .zip files to add them to the pack.')
			}
			return
		}
		try {
			const identified = await api.identifyDropped(store.pack!.dir, jars)
			if (identified.length) dropFiles = identified
			else store.notify('info', 'Nothing usable in that drop.')
		} catch (e) {
			store.notify('error', `${e}`)
		}
	}

	function onLink(inst: DetectedInstance) {
		showLink = false
		store.linkInstance(inst.gameDir)
	}

	function openFile(path: string) {
		showFind = false
		fileToOpen = path
		filesKey++
		store.setView('files')
	}

	$effect(() => {
		if (store.view !== 'files') fileToOpen = undefined
	})

	function onKey(e: KeyboardEvent) {
		if (!(e.metaKey || e.ctrlKey)) return
		if (e.key === ',') {
			e.preventDefault()
			store.settingsOpen = true
			return
		}
		if (!store.hasPack) return
		if (e.shiftKey && (e.key === 'p' || e.key === 'P')) {
			e.preventDefault()
			showPalette = true
		} else if (e.key === 'k' || e.key === 'K') {
			e.preventDefault()
			showAdd = true
		} else if (e.key === 'e' || e.key === 'E') {
			e.preventDefault()
			if (store.hasLock) showExport = true
		} else if (e.key === '1') {
			e.preventDefault()
			store.setView('content')
		} else if (e.key === '2') {
			e.preventDefault()
			store.setView('files')
		} else if (e.key === '3') {
			e.preventDefault()
			if (store.bound) store.setView('instance')
		} else if (e.key === '4') {
			e.preventDefault()
			store.setView('source')
		} else if (e.key === 'f' || e.key === 'F') {
			e.preventDefault()
			if (fileFind.open?.()) return
			showFind = true
		}
	}

	$effect(() => {
		document.title = store.pack ? `${store.pack.manifest.name} · packweave` : 'packweave'
	})

	function onContextMenu(e: MouseEvent) {
		const t = e.target as HTMLElement
		if (t.closest('input, textarea, [contenteditable="true"], [contenteditable=""]')) return
		e.preventDefault()
	}
</script>

<svelte:window onkeydown={onKey} oncontextmenu={onContextMenu} />

<div class="flex flex-col h-screen overflow-hidden bg-bg">
	{#if store.updateInfo}
		<div
			class="shrink-0 flex items-center justify-center gap-[0.8rem] px-4 py-[0.4rem] bg-brand text-on-brand text-[0.82rem]"
		>
			<span>packweave {store.updateInfo.version} is available.</span>
			<div class="flex items-center gap-[0.4rem]">
				<button
					class="bg-on-brand text-brand font-semibold text-[0.78rem] px-[0.6rem] py-[0.22rem] rounded-sm cursor-pointer disabled:opacity-60 disabled:cursor-default"
					disabled={store.busy}
					onclick={() => store.installUpdate()}>Install &amp; restart</button
				>
				<button
					class="bg-transparent text-on-brand text-[0.78rem] cursor-pointer opacity-85 hover:opacity-100"
					onclick={() => store.dismissUpdate()}>Later</button
				>
			</div>
		</div>
	{/if}

	{#if store.hasPack}
		<Toolbar
			onadd={() => (showAdd = true)}
			onexport={() => (showExport = true)}
			onsettings={() => (store.settingsOpen = true)}
			onlink={() => (showLink = true)}
		/>
		<main class="flex-1 min-h-0">
			{#if store.view === 'content'}
				<ContentView onadd={() => (showAdd = true)} />
			{:else if store.view === 'files'}
				{#key filesKey}
					<FilesView initialFile={fileToOpen} />
				{/key}
			{:else if store.view === 'instance'}
				<InstanceView onlink={() => (showLink = true)} />
			{:else if store.view === 'source'}
				<SourceView />
			{/if}
		</main>
	{:else}
		<WelcomeView onnew={() => (showStart = true)} />
	{/if}

	{#if showIntro}<IntroModal onstart={() => (showStart = true)} />{/if}
	{#if showStart}
		<StartPackModal onclose={() => (showStart = false)} onpick={onStartPick} />
	{/if}
	{#if showPickForNew}
		<InstancePicker onpick={onPickForNew} onclose={() => (showPickForNew = false)} />
	{/if}
	{#if showNew}
		<NewPackModal
			initialInstance={newInstance}
			onclose={() => {
				showNew = false
				newInstance = null
			}}
		/>
	{/if}
	{#if showClone}<CloneModal onclose={() => (showClone = false)} />{/if}
	{#if store.authPrompt}<AuthModal />{/if}
	{#if store.deletePrompt}<DeleteConfirm />{/if}
	{#if store.settingsOpen}<SettingsModal onclose={() => (store.settingsOpen = false)} />{/if}
	{#if showExport}<ExportModal onclose={() => (showExport = false)} />{/if}
	{#if showLink}
		<InstancePicker onpick={onLink} onclose={() => (showLink = false)} />
	{/if}
	{#if showAdd}<AddPalette onclose={() => (showAdd = false)} />{/if}
	{#if showPalette}
		<CommandPalette
			onclose={() => (showPalette = false)}
			onadd={() => (showAdd = true)}
			onexport={() => (showExport = true)}
		/>
	{/if}
	{#if dropFiles}
		<DropModal files={dropFiles} onclose={() => (dropFiles = null)} />
	{/if}
	{#if dragging}
		<div
			class="fixed inset-0 z-[80] grid place-items-center bg-[rgba(6,8,12,0.45)] backdrop-blur-[2px] pointer-events-none"
		>
			<div
				class="flex flex-col items-center gap-2 px-10 py-8 rounded-xl border-2 border-dashed border-brand bg-bg-super-raised/90 shadow-floating"
			>
				<span class="text-[1.05rem] font-semibold text-contrast">
					{store.hasPack ? 'Drop to add to the pack' : 'Drop a pack to import it'}
				</span>
				<span class="text-[0.8rem] text-secondary">
					{store.hasPack
						? 'Jars are matched to Modrinth / CurseForge automatically'
						: '.mrpack, CurseForge .zip, or Prism / MultiMC instance'}
				</span>
			</div>
		</div>
	{/if}
	{#if showFind}<SearchPanel {openFile} onclose={() => (showFind = false)} />{/if}
	<ToastHost />
	<ContextMenu />
</div>

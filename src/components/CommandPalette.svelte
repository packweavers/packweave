<script lang="ts">
	import {
		Boxes,
		BookOpen,
		FolderOpen,
		FolderTree,
		GitBranch,
		MonitorSmartphone,
		Moon,
		Package,
		Plus,
		RefreshCw,
		Settings,
		Terminal,
		X,
	} from '@lucide/svelte'
	import type { Component } from 'svelte'
	import { openGuides, revealFolder } from '../api'
	import { store } from '../lib/store.svelte'
	import { shortcut } from '../util'

	let {
		onclose,
		onadd,
		onexport,
	}: {
		onclose: () => void
		onadd: () => void
		onexport: () => void
	} = $props()

	interface Action {
		id: string
		label: string
		hint?: string
		icon: Component
		enabled: boolean
		run: () => void
	}

	const actions = $derived.by<Action[]>(() => [
		{
			id: 'view-content',
			label: 'Go to Content',
			hint: shortcut('1'),
			icon: Boxes,
			enabled: true,
			run: () => store.setView('content'),
		},
		{
			id: 'view-files',
			label: 'Go to Files',
			hint: shortcut('2'),
			icon: FolderTree,
			enabled: true,
			run: () => store.setView('files'),
		},
		{
			id: 'view-instance',
			label: 'Go to Instance',
			hint: shortcut('3'),
			icon: MonitorSmartphone,
			enabled: store.bound,
			run: () => store.setView('instance'),
		},
		{
			id: 'view-source',
			label: 'Go to Source control',
			hint: shortcut('4'),
			icon: GitBranch,
			enabled: true,
			run: () => store.setView('source'),
		},
		{
			id: 'add',
			label: 'Add content',
			hint: shortcut('K'),
			icon: Plus,
			enabled: true,
			run: onadd,
		},
		{
			id: 'export',
			label: 'Export / publish…',
			hint: shortcut('E'),
			icon: Package,
			enabled: store.hasLock,
			run: onexport,
		},
		{
			id: 'resolve',
			label: 'Re-resolve pack (refresh versions)',
			icon: RefreshCw,
			enabled: !store.busy,
			run: () => void store.runResolve(),
		},
		{
			id: 'reveal',
			label: 'Reveal pack folder',
			icon: FolderOpen,
			enabled: !!store.pack,
			run: () => {
				if (store.pack) void revealFolder(store.pack.dir)
			},
		},
		{
			id: 'copy-path',
			label: 'Copy pack path',
			icon: Terminal,
			enabled: !!store.pack,
			run: () => {
				if (store.pack) {
					navigator.clipboard?.writeText(store.pack.dir)
					store.notify('success', 'Path copied')
				}
			},
		},
		{
			id: 'theme',
			label: `Theme: ${store.theme} → ${store.theme === 'system' ? 'light' : store.theme === 'light' ? 'dark' : 'system'}`,
			icon: Moon,
			enabled: true,
			run: () =>
				store.setTheme(store.theme === 'system' ? 'light' : store.theme === 'light' ? 'dark' : 'system'),
		},
		{
			id: 'settings',
			label: 'Open Settings',
			hint: shortcut(','),
			icon: Settings,
			enabled: true,
			run: () => (store.settingsOpen = true),
		},
		{
			id: 'guides',
			label: 'Guides',
			icon: BookOpen,
			enabled: true,
			run: () => void openGuides(),
		},
		{
			id: 'close-pack',
			label: 'Close pack',
			icon: X,
			enabled: !!store.pack,
			run: () => store.closePack(),
		},
	])

	let query = $state('')
	let active = $state(0)

	const shown = $derived.by(() => {
		const q = query.trim().toLowerCase()
		const pool = actions.filter((a) => a.enabled)
		if (!q) return pool
		return pool.filter((a) => a.label.toLowerCase().includes(q))
	})

	function run(a: Action) {
		onclose()
		a.run()
	}

	function onKeydown(e: KeyboardEvent) {
		if (e.key === 'ArrowDown') {
			e.preventDefault()
			active = Math.min(active + 1, shown.length - 1)
		} else if (e.key === 'ArrowUp') {
			e.preventDefault()
			active = Math.max(active - 1, 0)
		} else if (e.key === 'Enter') {
			e.preventDefault()
			const a = shown[Math.min(active, shown.length - 1)]
			if (a) run(a)
		}
	}
</script>

<svelte:window onkeydown={(e) => e.key === 'Escape' && onclose()} />

<div
	class="fixed inset-0 z-[65] flex justify-center items-start pt-[14vh] bg-[rgba(6,8,12,0.4)] backdrop-blur-[2px]"
	role="presentation"
	onclick={(e) => {
		if (e.target === e.currentTarget) onclose()
	}}
>
	<div
		class="w-[560px] max-w-[calc(100vw-2rem)] max-h-[62vh] flex flex-col bg-bg-super-raised border border-divider rounded-lg shadow-floating overflow-hidden"
	>
		<div class="flex items-center gap-2.5 px-4 py-3 border-b border-divider text-secondary">
			<Terminal size={17} />
			<!-- svelte-ignore a11y_autofocus -->
			<input
				bind:value={query}
				oninput={() => (active = 0)}
				onkeydown={onKeydown}
				class="flex-1 bg-transparent border-none text-contrast text-large outline-none"
				placeholder="Run a command…"
				autofocus
				spellcheck="false"
			/>
			<kbd class="text-micro bg-button-bg border border-divider rounded-sm px-sm py-2xs text-secondary"
				>esc</kbd
			>
		</div>
		<div class="overflow-y-auto p-md">
			{#if !shown.length}
				<div class="text-center text-secondary py-5xl px-4 text-regular">No matching command.</div>
			{/if}
			{#each shown as a, i (a.id)}
				{@const Icon = a.icon}
				<button
					class={`flex items-center gap-2.5 w-full text-left border-none px-3 py-lg rounded-md cursor-pointer ${
						i === active ? 'bg-button-bg text-contrast' : 'bg-transparent text-body hover:bg-button-bg'
					}`}
					onclick={() => run(a)}
					onmouseenter={() => (active = i)}
				>
					<Icon size={16} class="text-secondary shrink-0" />
					<span class="flex-1 min-w-0 text-regular">{a.label}</span>
					{#if a.hint}
						<kbd
							class="text-micro bg-bg-inset border border-divider rounded-sm px-sm py-hair text-secondary"
							>{a.hint}</kbd
						>
					{/if}
				</button>
			{/each}
		</div>
	</div>
</div>

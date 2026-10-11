<script lang="ts">
	import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow'
	import { getCurrentWindow } from '@tauri-apps/api/window'
	import { onMount } from 'svelte'

	import { windowState } from '@/lib/state/window.svelte'
	import { isMac } from '@/lib/util/platform'
	import WindowControls from '@/shell/WindowControls.svelte'
	import DialogHost from '@/ui/DialogHost.svelte'
	import Toasts from '@/ui/Toasts.svelte'

	import AboutPage from './AboutPage.svelte'
	import AccountsPage from './AccountsPage.svelte'
	import AppearancePage from './AppearancePage.svelte'
	import GitPage from './GitPage.svelte'
	import KeyboardPage from './KeyboardPage.svelte'
	import LaunchersPage from './LaunchersPage.svelte'
	import StoragePage from './StoragePage.svelte'
	import UpdatesPage from './UpdatesPage.svelte'

	type Page =
		'appearance' | 'accounts' | 'git' | 'launchers' | 'keyboard' | 'updates' | 'storage' | 'about'

	const PAGES: { id: Page; label: string }[] = [
		{ id: 'appearance', label: 'Appearance' },
		{ id: 'accounts', label: 'Accounts and tokens' },
		{ id: 'git', label: 'Git' },
		{ id: 'launchers', label: 'Launchers and instances' },
		{ id: 'keyboard', label: 'Keyboard' },
		{ id: 'updates', label: 'Updates' },
		{ id: 'storage', label: 'Storage' },
		{ id: 'about', label: 'About' },
	]

	const initial = new URLSearchParams(location.search).get('page') as Page | null
	let page = $state<Page>(initial && PAGES.some((p) => p.id === initial) ? initial : 'appearance')

	onMount(() => {
		if (!isMac) void getCurrentWindow().show()
		const stopFollowing = windowState.follow()
		const stop = getCurrentWebviewWindow().listen<string>('settings-page', (event) => {
			if (PAGES.some((p) => p.id === event.payload)) page = event.payload as Page
		})
		return () => {
			stopFollowing()
			void stop.then((unlisten) => unlisten())
		}
	})

	function onkeydown(event: KeyboardEvent): void {
		if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === 'w') {
			event.preventDefault()
			void getCurrentWindow().close()
		}
	}
</script>

<svelte:window {onkeydown} />

<div class="window">
	<header class="title-bar" class:inset={isMac && !windowState.fullscreen} data-tauri-drag-region>
		<span class="title" data-tauri-drag-region>Settings</span>
		<span class="spacer" data-tauri-drag-region></span>
		{#if !isMac}<WindowControls />{/if}
	</header>
	<div class="body">
		<nav class="nav">
			{#each PAGES as item (item.id)}
				<button class="nav-item" class:on={page === item.id} onclick={() => (page = item.id)}
					>{item.label}</button
				>
			{/each}
		</nav>
		<main class="scroll page">
			{#if page === 'appearance'}
				<AppearancePage />
			{:else if page === 'accounts'}
				<AccountsPage />
			{:else if page === 'git'}
				<GitPage />
			{:else if page === 'launchers'}
				<LaunchersPage />
			{:else if page === 'keyboard'}
				<KeyboardPage />
			{:else if page === 'updates'}
				<UpdatesPage />
			{:else if page === 'storage'}
				<StoragePage />
			{:else}
				<AboutPage />
			{/if}
		</main>
	</div>
</div>
<DialogHost />
<Toasts />

<style>
	.window {
		height: 100%;
		display: flex;
		flex-direction: column;
	}

	.title-bar {
		flex: none;
		height: var(--title-bar);
		display: flex;
		align-items: stretch;
		padding-left: 14px;
		background: var(--chrome);
		border-bottom: 1px solid var(--line);
	}

	.title-bar.inset {
		padding-left: calc(80px / var(--ui-zoom, 1));
	}

	.title {
		align-self: center;
		font-weight: 600;
	}

	.body {
		flex: 1;
		min-height: 0;
		display: flex;
	}

	.nav {
		flex: 0 0 210px;
		display: flex;
		flex-direction: column;
		padding: 8px 0;
		background: var(--chrome);
		border-right: 1px solid var(--line);
	}

	.nav-item {
		height: var(--row);
		padding: 0 16px;
		text-align: left;
	}

	.nav-item:hover {
		background: var(--hover);
	}

	.nav-item.on {
		background: var(--selected);
		font-weight: 600;
	}

	.page {
		padding-bottom: 32px;
	}

	.page :global(.setting) {
		display: flex;
		align-items: center;
		gap: 10px;
		min-height: 44px;
		padding: 6px 16px;
		border-bottom: 1px solid var(--line);
	}

	.page :global(.key) {
		flex: 0 0 180px;
		font-weight: 500;
	}

	.page :global(.grow) {
		flex: 1;
		max-width: 420px;
	}
</style>

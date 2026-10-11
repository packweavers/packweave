<script lang="ts">
	import { PanelRight, Search } from '@lucide/svelte'

	import { docTitle } from '@/documents/registry'
	import { git } from '@/lib/state/git.svelte'
	import { layout } from '@/lib/state/layout.svelte'
	import { palette } from '@/lib/state/palette.svelte'
	import { tabs } from '@/lib/state/tabs.svelte'
	import { windowState } from '@/lib/state/window.svelte'
	import { ws } from '@/lib/state/workspace.svelte'
	import { setWindowTitle } from '@/lib/tauri/windows'
	import { isMac } from '@/lib/util/platform'
	import IconButton from '@/ui/IconButton.svelte'

	import Editor from './Editor.svelte'
	import Inspector from './Inspector.svelte'
	import Palette from './Palette.svelte'
	import WindowControls from './WindowControls.svelte'

	const title = $derived(tabs.active ? `${docTitle(tabs.active)} — ${ws.name}` : ws.name)

	$effect(() => {
		void setWindowTitle(title)
	})
</script>

<header class="title-bar" class:inset={isMac && !windowState.fullscreen} data-tauri-drag-region>
	<span class="where" data-tauri-drag-region>
		<span class="ellipsis">{ws.name}</span>
		<span>/</span>
		<span class="ellipsis">{git.branch ?? (git.isRepo ? 'detached' : 'no git')}</span>
	</span>
	{#if palette.open}
		<Palette />
	{/if}
	<div class="right" data-tauri-drag-region>
		<IconButton
			icon={Search}
			label="Search"
			command="palette.open"
			onclick={() => palette.show()}
		/>
		<IconButton
			icon={PanelRight}
			label="Toggle inspector"
			command="layout.inspector"
			active={layout.inspector}
			onclick={() => layout.toggleInspector()}
		/>
		{#if !isMac}
			<WindowControls />
		{/if}
	</div>
</header>
<div class="body">
	<div class="center">
		<Editor />
	</div>
	{#if layout.inspector}
		<Inspector />
	{/if}
</div>

<style>
	.title-bar {
		position: relative;
		flex: none;
		height: var(--title-bar);
		display: flex;
		align-items: stretch;
		padding-left: 12px;
		background: var(--chrome);
		border-bottom: 1px solid var(--line);
	}

	.title-bar.inset {
		padding-left: calc(80px / var(--ui-zoom, 1));
	}

	.where {
		min-width: 0;
		display: flex;
		align-items: center;
		gap: 6px;
		color: var(--grey);
		font-size: 12.5px;
		white-space: nowrap;
	}

	.right {
		margin-left: auto;
		display: flex;
		align-items: stretch;
		gap: 1px;
	}

	.right > :global(.icon-btn) {
		width: 40px;
		height: auto;
	}

	.body {
		flex: 1;
		min-height: 0;
		display: flex;
	}

	.center {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
	}
</style>

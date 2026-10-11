<script lang="ts">
	import { ChevronDown, PanelBottom, PanelLeft, PanelRight, Search, Settings } from '@lucide/svelte'

	import { packMenu } from '@/lib/menus'
	import { keysFor, runCommand } from '@/lib/state/commands.svelte'
	import { git } from '@/lib/state/git.svelte'
	import { instance } from '@/lib/state/instance.svelte'
	import { layout } from '@/lib/state/layout.svelte'
	import { menus } from '@/lib/state/menus.svelte'
	import { palette } from '@/lib/state/palette.svelte'
	import { windowState } from '@/lib/state/window.svelte'
	import { ws } from '@/lib/state/workspace.svelte'
	import { openSettingsWindow } from '@/lib/tauri/windows'
	import { isMac } from '@/lib/util/platform'
	import Icon from '@/ui/Icon.svelte'
	import IconButton from '@/ui/IconButton.svelte'
	import Popover from '@/ui/Popover.svelte'

	import BranchPicker from './BranchPicker.svelte'
	import InstanceDot from './InstanceDot.svelte'
	import Palette from './Palette.svelte'
	import WindowControls from './WindowControls.svelte'

	let branchChip = $state<HTMLButtonElement | null>(null)
	let picking = $state(false)

	const searchKeys = $derived(keysFor('palette.open'))
</script>

<header class="title-bar" class:inset={isMac && !windowState.fullscreen} data-tauri-drag-region>
	{#if ws.isOpen}
		<button
			class="chip"
			class:on={menus.current !== null}
			onclick={(e) => menus.below(e.currentTarget, packMenu())}
		>
			<span class="ellipsis">{ws.name}</span>
			<Icon icon={ChevronDown} size={12} />
		</button>
		<span class="slash" data-tauri-drag-region>/</span>
		<button
			bind:this={branchChip}
			class="chip"
			class:on={picking}
			onclick={() => (picking = !picking)}
		>
			<InstanceDot status={instance.status} />
			<span class="ellipsis">{git.branch ?? (git.isRepo ? 'detached' : 'no git')}</span>
			{#if git.merging}<span class="tag changed">merging</span>{/if}
			<Icon icon={ChevronDown} size={12} />
		</button>
	{:else}
		<span class="app-name" data-tauri-drag-region>packweave</span>
	{/if}

	{#if palette.open}
		<Palette />
	{:else}
		<button class="search" onclick={() => palette.show()}>
			<Icon icon={Search} size={14} />
			<span class="placeholder">Search</span>
			{#if searchKeys}<span class="kbd">{searchKeys}</span>{/if}
		</button>
	{/if}

	<div class="right" data-tauri-drag-region>
		{#if ws.isOpen}
			<button class="btn" onclick={() => runCommand('release.open')}>Export & Release</button>
			<IconButton
				icon={PanelLeft}
				label="Toggle sidebar"
				command="layout.sidebar"
				active={layout.sidebar}
				onclick={() => layout.toggleSidebar()}
			/>
			<IconButton
				icon={PanelBottom}
				label="Toggle bottom panel"
				command="layout.panel"
				active={layout.panel}
				onclick={() => layout.togglePanel()}
			/>
			<IconButton
				icon={PanelRight}
				label="Toggle inspector"
				command="layout.inspector"
				active={layout.inspector}
				onclick={() => layout.toggleInspector()}
			/>
		{:else}
			<IconButton
				icon={Settings}
				label="Settings"
				command="app.settings"
				onclick={() => void openSettingsWindow()}
			/>
		{/if}
		{#if !isMac}
			<WindowControls />
		{/if}
	</div>
</header>

{#if picking && branchChip}
	<Popover anchor={branchChip} width={320} onclose={() => (picking = false)}>
		<BranchPicker onclose={() => (picking = false)} />
	</Popover>
{/if}

<style>
	.title-bar {
		position: relative;
		flex: none;
		height: var(--title-bar);
		display: flex;
		align-items: stretch;
		padding-left: 8px;
		background: var(--chrome);
		border-bottom: 1px solid var(--line);
	}

	.title-bar.inset {
		padding-left: calc(80px / var(--ui-zoom, 1));
	}

	.chip {
		display: inline-flex;
		align-items: center;
		gap: 7px;
		max-width: 260px;
		padding: 0 10px;
		font-weight: 500;
		white-space: nowrap;
	}

	.chip:hover,
	.chip.on {
		background: var(--selected);
	}

	.slash {
		align-self: center;
		padding: 0 2px;
	}

	.app-name {
		align-self: center;
		padding: 0 10px;
		font-weight: 600;
	}

	.search {
		position: absolute;
		left: 50%;
		top: 0;
		bottom: 0;
		transform: translateX(-50%);
		width: min(380px, 34vw);
		display: flex;
		align-items: center;
		gap: 8px;
		padding-left: 12px;
		background: var(--band);
		font-size: 12.5px;
	}

	.search:hover {
		background: var(--fill);
	}

	.search .placeholder {
		flex: 1;
		text-align: left;
		font-style: italic;
		opacity: 0.5;
	}

	.search .kbd {
		align-self: stretch;
		min-width: 40px;
	}

	.right {
		margin-left: auto;
		display: flex;
		align-items: stretch;
		gap: 1px;
	}

	.right > :global(.btn) {
		height: auto;
	}

	.right > :global(.btn + .icon-btn) {
		margin-left: 9px;
	}

	.right > :global(.icon-btn) {
		width: 40px;
		height: auto;
	}
</style>

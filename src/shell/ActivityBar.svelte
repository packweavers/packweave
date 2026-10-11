<script lang="ts">
	import { Blocks, Bug, Files, GitBranch, Search, Settings } from '@lucide/svelte'

	import { bisect } from '@/lib/state/bisect.svelte'
	import { content } from '@/lib/state/content.svelte'
	import { git } from '@/lib/state/git.svelte'
	import { layout, type ViewId } from '@/lib/state/layout.svelte'
	import { openSettingsWindow } from '@/lib/tauri/windows'
	import IconButton from '@/ui/IconButton.svelte'

	const errors = $derived(content.problems.filter((p) => p.severity === 'error').length)
	const views = $derived([
		{
			id: 'content' as ViewId,
			icon: Blocks,
			label: 'Content',
			command: 'view.content',
			badge: content.problems.filter((p) => p.severity !== 'info').length,
			tone: errors > 0 ? ('error' as const) : ('warn' as const),
		},
		{
			id: 'files' as ViewId,
			icon: Files,
			label: 'Files',
			command: 'view.files',
			badge: 0,
			tone: null,
		},
		{
			id: 'search' as ViewId,
			icon: Search,
			label: 'Search',
			command: 'view.search',
			badge: 0,
			tone: null,
		},
		{
			id: 'git' as ViewId,
			icon: GitBranch,
			label: 'Git',
			command: 'view.git',
			badge: git.changes.length,
			tone: null,
		},
		{
			id: 'debug' as ViewId,
			icon: Bug,
			label: 'Debug',
			command: 'view.debug',
			badge: bisect.session && !bisect.session.found ? 1 : 0,
			tone: 'warn' as const,
		},
	])
</script>

<nav class="activity-bar" aria-label="Views">
	{#each views as view (view.id)}
		<IconButton
			icon={view.icon}
			label={view.label}
			command={view.command}
			size={20}
			active={layout.sidebar && layout.view === view.id}
			badge={view.badge}
			badgeTone={view.tone}
			onclick={() => layout.pickView(view.id)}
		/>
	{/each}
	<span class="spacer"></span>
	<IconButton
		icon={Settings}
		label="Settings"
		command="app.settings"
		size={20}
		onclick={() => void openSettingsWindow()}
	/>
</nav>

<style>
	.activity-bar {
		position: relative;
		width: var(--activity-bar);
		flex: none;
		display: flex;
		flex-direction: column;
		background: var(--chrome);
		border-right: 1px solid var(--line);
	}

	.activity-bar :global(.icon-btn) {
		width: auto;
		height: 42px;
	}

	.activity-bar :global(.icon-btn.on::before) {
		content: '';
		position: absolute;
		left: 0;
		top: 0;
		bottom: 0;
		width: 2px;
		background: var(--text);
	}

	.activity-bar :global(.count) {
		right: 5px;
		top: 5px;
	}
</style>

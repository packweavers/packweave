<script lang="ts">
	import { layout } from '@/lib/state/layout.svelte'
	import { panels } from '@/lib/state/panels.svelte'

	import ActivityBar from './ActivityBar.svelte'
	import Editor from './Editor.svelte'
	import Inspector from './Inspector.svelte'
	import Panel from './Panel.svelte'
	import Sidebar from './Sidebar.svelte'
	import StatusBar from './StatusBar.svelte'
</script>

<div class="body">
	<ActivityBar />
	{#if layout.sidebar}
		<Sidebar />
	{/if}
	<div class="center">
		<Editor />
		{#if layout.panel && panels.shown.length > 0}
			<Panel />
		{/if}
	</div>
	{#if layout.inspector}
		<Inspector />
	{/if}
</div>
<StatusBar />

<style>
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

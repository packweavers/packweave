<script lang="ts">
	import { layout } from '@/lib/state/layout.svelte'
	import Splitter from '@/ui/Splitter.svelte'
	import ContentView from '@/views/ContentView.svelte'
	import DebugView from '@/views/DebugView.svelte'
	import FilesView from '@/views/FilesView.svelte'
	import GitView from '@/views/GitView.svelte'
	import SearchView from '@/views/SearchView.svelte'
</script>

<aside class="sidebar" style:width="{layout.sidebarWidth}px" data-busy-scope="view:{layout.view}">
	{#if layout.view === 'content'}
		<ContentView />
	{:else if layout.view === 'files'}
		<FilesView />
	{:else if layout.view === 'search'}
		<SearchView />
	{:else if layout.view === 'debug'}
		<DebugView />
	{:else}
		<GitView />
	{/if}
	<div class="edge">
		<Splitter
			axis="x"
			bind:value={layout.sidebarWidth}
			min={200}
			max={480}
			ondone={() => layout.persist()}
		/>
	</div>
</aside>

<style>
	.sidebar {
		position: relative;
		flex: none;
		display: flex;
		flex-direction: column;
		min-height: 0;
		background: var(--chrome);
		border-right: 1px solid var(--line);
	}

	.edge {
		position: absolute;
		top: 0;
		bottom: 0;
		right: 0;
	}
</style>

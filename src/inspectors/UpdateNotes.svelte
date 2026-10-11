<script lang="ts">
	import { ChevronDown, ChevronRight } from '@lucide/svelte'

	import { ws } from '@/lib/state/workspace.svelte'
	import { updateChangelogs } from '@/lib/tauri/catalog'
	import type { VersionNotes } from '@/lib/tauri/types'
	import { ago } from '@/lib/util/format'
	import Icon from '@/ui/Icon.svelte'
	import Markdown from '@/ui/Markdown.svelte'

	interface Props {
		projectId: string
		version: string
		expandAll?: boolean
	}

	let { projectId, version, expandAll = false }: Props = $props()

	let notes = $state<VersionNotes[] | null>(null)
	let failed = $state(false)
	let open = $state<Record<string, boolean>>({})

	$effect(() => {
		const dir = ws.dir
		const id = projectId
		const to = version
		if (!dir) return
		notes = null
		failed = false
		let live = true
		void updateChangelogs(dir, id, to)
			.then((list) => {
				if (live) notes = list
			})
			.catch(() => {
				if (!live) return
				notes = []
				failed = true
			})
		return () => {
			live = false
		}
	})

	function shown(note: VersionNotes, index: number): boolean {
		return open[note.versionId] ?? (expandAll || index === 0)
	}
</script>

{#if notes === null}
	<div class="loading"><span class="spinner"></span></div>
{:else if notes.length === 0}
	<p class="small hint">
		{failed ? "Couldn't load the changelog." : 'No changelog for this update.'}
	</p>
{:else}
	<div class="notes">
		{#each notes as note, i (note.versionId)}
			{@const expanded = shown(note, i)}
			<div class="note">
				<button
					class="note-head"
					aria-expanded={expanded}
					onclick={() => (open[note.versionId] = !expanded)}
				>
					<Icon icon={expanded ? ChevronDown : ChevronRight} size={12} />
					<span class="mono ellipsis">{note.versionNumber}</span>
					{#if note.versionType !== 'release'}<span class="tag changed">{note.versionType}</span
						>{/if}
					<span class="spacer"></span>
					{#if note.datePublished}<span class="small hint">{ago(note.datePublished)}</span>{/if}
				</button>
				{#if expanded}
					<div class="note-body">
						{#if note.changelog.trim()}
							<Markdown source={note.changelog} html={note.html} />
						{:else}
							<p class="small hint">No changelog for this version.</p>
						{/if}
					</div>
				{/if}
			</div>
		{/each}
	</div>
{/if}

<style>
	.loading {
		padding: 10px 0;
	}

	.notes {
		display: flex;
		flex-direction: column;
	}

	.note + .note {
		border-top: 1px solid var(--line);
	}

	.note-head {
		width: 100%;
		min-width: 0;
		display: flex;
		align-items: center;
		gap: 6px;
		min-height: 30px;
		text-align: left;
	}

	.note-head:hover {
		background: var(--hover);
	}

	.note-body {
		padding: 2px 0 10px 18px;
	}
</style>

<script lang="ts">
	import { onMount } from 'svelte'

	import { renderChangelog, summarize } from '@/lib/content/changelog'
	import { git } from '@/lib/state/git.svelte'
	import { release } from '@/lib/state/release.svelte'
	import { tabs, type Doc } from '@/lib/state/tabs.svelte'
	import { tasks } from '@/lib/state/tasks.svelte'
	import { toasts } from '@/lib/state/toasts.svelte'
	import { ws } from '@/lib/state/workspace.svelte'
	import { compare, latestTag, log, packDiff, saveChangelog } from '@/lib/tauri/git'
	import type { PackDiff } from '@/lib/tauri/types'
	import Segmented from '@/ui/Segmented.svelte'

	interface Props {
		doc: Doc
	}

	let {}: Props = $props()

	const EMPTY_TREE = '4b825dc642cb6eb9a060e54bf8d69288fbee4904'

	let tag = $state<string | null>(null)
	let diff = $state<PackDiff | null>(null)
	let subjects = $state<string[]>([])

	const title = $derived(`${ws.name} v${release.version || ws.manifest?.version || ''}`.trim())

	onMount(() => {
		const dir = ws.dir
		if (!dir) return
		void latestTag(dir)
			.then((t) => (tag = t))
			.catch(tasks.or('Reading the latest release tag', undefined))
	})

	$effect(() => {
		const dir = ws.dir
		const since = release.range === 'release' ? tag : null
		void git.head
		if (!dir) return
		if (since) {
			void compare(dir, since, 'HEAD')
				.then((result) => {
					diff = result.pack
					subjects = result.commits.map((c) => c.subject)
				})
				.catch((error) => {
					diff = null
					tasks.fail('Comparing with the last release', error)
				})
		} else {
			void packDiff(dir, EMPTY_TREE, 'HEAD')
				.then((d) => (diff = d))
				.catch((error) => {
					diff = null
					tasks.fail('Reading the pack history', error)
				})
			void log(dir, 200)
				.then((commits) => (subjects = commits.map((c) => c.subject)))
				.catch(tasks.or('Reading commits', undefined))
		}
	})

	$effect(() => {
		const current = diff
		const options = { ...release.options }
		if (!current || release.changelogEdited) return
		release.changelogText = renderChangelog(current, subjects, title, options)
	})

	function regenerate(): void {
		release.changelogEdited = false
	}

	async function save(): Promise<void> {
		if (!ws.dir) return
		try {
			await saveChangelog(ws.dir, release.changelogText)
			ws.changed()
			toasts.show('Saved CHANGELOG.md')
		} catch (error) {
			toasts.error(error)
		}
	}

	function useInRelease(): void {
		release.notes = release.changelogText.replace(/^##[^\n]*\n+/, '')
		tabs.open('release')
	}

	function copy(): void {
		void navigator.clipboard
			.writeText(release.changelogText)
			.then(() => toasts.show('Copied'))
			.catch((error) => toasts.error(error))
	}
</script>

<div class="bar doc-toolbar">
	<span class="doc-title">Changelog</span>
	<span class="mono small"
		>{release.range === 'release' && tag ? tag : 'start'} → {git.branch ?? 'HEAD'}</span
	>
	<Segmented
		small
		options={[
			{ value: 'release', label: 'Since last release' },
			{ value: 'all', label: 'Whole pack' },
		]}
		bind:value={release.range}
		onchange={regenerate}
	/>
	<span class="spacer"></span>
	{#if release.changelogEdited}<button class="btn" onclick={regenerate}>Regenerate</button>{/if}
	<button class="btn" onclick={copy}>Copy</button>
	<button class="btn" onclick={save}>Save</button>
	<button class="btn primary" onclick={useInRelease}>Use in release</button>
</div>
{#if diff}
	<div class="summary small">{summarize(diff) || 'No pack changes'}</div>
{/if}
<textarea
	class="text code"
	spellcheck="true"
	bind:value={release.changelogText}
	oninput={() => (release.changelogEdited = true)}></textarea>

<style>
	.summary {
		flex: none;
		padding: 6px 16px;
		border-bottom: 1px solid var(--line);
	}

	.text {
		flex: 1;
		min-height: 0;
		padding: 16px 20px;
		resize: none;
		background: var(--doc);
		white-space: pre-wrap;
		user-select: text;
		-webkit-user-select: text;
	}

	.text:focus-visible {
		outline: none;
	}
</style>

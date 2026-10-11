<script lang="ts">
	import { onMount } from 'svelte'

	import { displayPath } from '@/lib/git/changes'
	import type { Doc } from '@/lib/state/tabs.svelte'
	import { toasts } from '@/lib/state/toasts.svelte'
	import { ws } from '@/lib/state/workspace.svelte'
	import { showDiff } from '@/lib/tauri/git'
	import DiffView from '@/ui/DiffView.svelte'
	import Segmented from '@/ui/Segmented.svelte'

	interface Props {
		doc: Doc
	}

	let { doc }: Props = $props()

	const hash = $derived(doc.arg.slice(0, doc.arg.indexOf(':')))
	const path = $derived(doc.arg.slice(doc.arg.indexOf(':') + 1))

	let diff = $state('')
	let mode = $state<'unified' | 'split'>('unified')

	onMount(() => {
		if (!ws.dir) return
		void showDiff(ws.dir, hash, path)
			.then((d) => (diff = d))
			.catch((error) => toasts.error(error))
	})
</script>

<div class="bar doc-toolbar">
	<span class="doc-title mono">{displayPath(path)}</span>
	<span class="mono small">{hash.slice(0, 7)}</span>
	<span class="spacer"></span>
	<Segmented
		options={[
			{ value: 'unified', label: 'Unified' },
			{ value: 'split', label: 'Side by side' },
		]}
		bind:value={mode}
	/>
</div>
<div class="scroll"><DiffView {diff} split={mode === 'split'} /></div>

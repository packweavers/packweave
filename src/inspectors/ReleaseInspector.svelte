<script lang="ts">
	import { untrack } from 'svelte'

	import { publishSteps } from '@/lib/flows/release'
	import { app } from '@/lib/state/app.svelte'
	import { git } from '@/lib/state/git.svelte'
	import { release } from '@/lib/state/release.svelte'
	import type { Doc } from '@/lib/state/tabs.svelte'
	import { tasks } from '@/lib/state/tasks.svelte'
	import { ws } from '@/lib/state/workspace.svelte'

	interface Props {
		doc: Doc
	}

	let {}: Props = $props()

	let steps = $state<string[]>([])

	const main = $derived(ws.mainDir ?? ws.dir ?? '')
	const version = $derived(release.version || '1.0.0')

	$effect(() => {
		const next = version
		void $state.snapshot(app.pack(main))
		untrack(
			() =>
				void publishSteps(next)
					.then((found) => (steps = found))
					.catch((error) => tasks.fail('Listing the publish steps', error)),
		)
	})
</script>

<div class="inspector-head">
	<div class="heading">
		<span class="strong">{ws.name} v{release.version || '…'}</span>
		<span class="small">{release.channel}</span>
	</div>
</div>
<div class="inspector-body">
	<section class="inspector-section">
		<div class="band">Publish will</div>
		{#each steps as step, i (step)}
			<div class="item-row"><span class="mono small">{i + 1}</span>{step}</div>
		{:else}
			<span class="small">Pick where to publish</span>
		{/each}
	</section>
	<section class="inspector-section">
		<div class="band">Releases</div>
		{#each git.tags.slice(0, 10) as tag (tag.name)}
			<div class="item-row">
				<span class="mono">{tag.name}</span><span class="spacer"></span><span class="small ellipsis"
					>{tag.subject}</span
				>
			</div>
		{:else}
			<span class="small">None yet</span>
		{/each}
	</section>
</div>

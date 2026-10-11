<script lang="ts">
	import { release } from '@/lib/state/release.svelte'
	import type { Doc } from '@/lib/state/tabs.svelte'
	import Segmented from '@/ui/Segmented.svelte'

	interface Props {
		doc: Doc
	}

	let {}: Props = $props()

	function changed(): void {
		release.changelogEdited = false
	}
</script>

<div class="inspector-head">
	<div class="heading"><span class="strong">Changelog</span></div>
</div>
<div class="inspector-body">
	<section class="inspector-section">
		<div class="band">Include</div>
		<label class="check"
			><input type="checkbox" bind:checked={release.options.versions} onchange={changed} />Version
			numbers</label
		>
		<label class="check"
			><input
				type="checkbox"
				bind:checked={release.options.nestDependencies}
				onchange={changed}
			/>Dependencies under their parent</label
		>
		<label class="check"
			><input type="checkbox" bind:checked={release.options.configs} onchange={changed} />Config
			changes</label
		>
		<label class="check"
			><input type="checkbox" bind:checked={release.options.commits} onchange={changed} />Commit
			messages</label
		>
	</section>
	<section class="inspector-section">
		<div class="band">Format</div>
		<Segmented
			options={[
				{ value: 'markdown', label: 'Markdown' },
				{ value: 'plain', label: 'Plain' },
				{ value: 'discord', label: 'Discord' },
			]}
			bind:value={release.options.format}
			onchange={changed}
		/>
	</section>
</div>

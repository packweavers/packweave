<script lang="ts">
	import { Bug, FlaskConical } from '@lucide/svelte'

	import { newTestBranch } from '@/lib/flows/branches'
	import { bisect } from '@/lib/state/bisect.svelte'
	import { git } from '@/lib/state/git.svelte'
	import { tabs } from '@/lib/state/tabs.svelte'
	import { plural } from '@/lib/util/format'
	import Icon from '@/ui/Icon.svelte'
	import { pending } from '@/ui/pending'

	const progress = $derived.by(() => {
		const session = bisect.session
		if (!session) return null
		if (session.found) return 'Finished'
		return `Step ${session.history.length + 1} · ${plural(session.suspects.length, 'suspect', 'suspects')} left`
	})
</script>

<div class="bar view-header"><span>Debug</span></div>
<div class="scroll">
	<section class="tool">
		<div class="head">
			<Icon icon={Bug} size={16} /><span class="medium">Find a broken mod</span>
		</div>
		<p class="small hint">Binary search mods to find the culprit.</p>
		{#if progress}<span class="small">{progress}</span>{/if}
		<button class="btn" onclick={() => tabs.open('bisect')}
			>{bisect.session ? 'Continue' : 'Start'}</button
		>
	</section>
	<section class="tool">
		<div class="head">
			<Icon icon={FlaskConical} size={16} /><span class="medium">Test in a branch</span>
		</div>
		<p class="small hint">Test pack changes in a separate branch before committing.</p>
		<button class="btn" disabled={!git.isRepo} onclick={pending(() => newTestBranch())}
			>New test branch…</button
		>
		{#if !git.isRepo}<span class="small hint">Start using Git first</span>{/if}
	</section>
</div>

<style>
	.tool {
		display: flex;
		flex-direction: column;
		align-items: flex-start;
		gap: 8px;
		padding: 14px 16px;
		border-bottom: 1px solid var(--line);
	}

	.head {
		display: flex;
		align-items: center;
		gap: 8px;
	}
</style>

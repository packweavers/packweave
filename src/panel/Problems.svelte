<script lang="ts">
	import { CircleAlert, Info, TriangleAlert } from '@lucide/svelte'

	import { removeContent, updatePins } from '@/lib/flows/content'
	import { content } from '@/lib/state/content.svelte'
	import { tabs } from '@/lib/state/tabs.svelte'
	import type { Fix, Validation } from '@/lib/tauri/types'
	import { typeName } from '@/lib/util/format'
	import Icon from '@/ui/Icon.svelte'

	let filter = $state<'all' | 'error' | 'warning'>('all')

	const problems = $derived(content.problems)
	const errors = $derived(problems.filter((p) => p.severity === 'error').length)
	const warnings = $derived(problems.filter((p) => p.severity === 'warning').length)
	const shown = $derived(
		problems
			.filter((p) => filter === 'all' || p.severity === filter)
			.sort((a, b) => rank(a) - rank(b)),
	)

	function rank(v: Validation): number {
		return { error: 0, warning: 1, info: 2, ok: 3 }[v.severity]
	}

	function apply(fix: Fix): void {
		switch (fix.kind) {
			case 'remove':
				void removeContent([fix.projectId])
				break
			case 'update':
				if (fix.version) void updatePins([{ key: fix.projectId, version: fix.version }])
				break
			case 'enable':
				void content.setDisabled([fix.projectId], false)
				break
			case 'unpin':
				void content.setPinned([fix.projectId], false)
				break
		}
	}
</script>

<div class="bar sub-toolbar">
	<button class="chip" class:on={filter === 'all'} onclick={() => (filter = 'all')}>All</button>
	<button class="chip" class:on={filter === 'error'} onclick={() => (filter = 'error')}>
		<Icon icon={CircleAlert} size={12} /><span class="count-text">{errors}</span>
	</button>
	<button class="chip" class:on={filter === 'warning'} onclick={() => (filter = 'warning')}>
		<Icon icon={TriangleAlert} size={12} /><span class="count-text">{warnings}</span>
	</button>
	<span class="spacer"></span>
	<button
		class="btn"
		disabled={content.resolving}
		aria-busy={content.resolving}
		onclick={() => void content.resolve(true)}>Check again</button
	>
</div>
<div class="scroll">
	{#each shown as problem (problem.id)}
		{@const mod = problem.projectId ? content.find(problem.projectId) : undefined}
		<div class="trow">
			<span class="severity {problem.severity}">
				<Icon
					icon={problem.severity === 'error'
						? CircleAlert
						: problem.severity === 'warning'
							? TriangleAlert
							: Info}
					size={14}
				/>
			</span>
			<span class="name">{problem.title}</span>
			{#if problem.detail}<span class="small ellipsis detail">{problem.detail}</span>{/if}
			<span class="spacer"></span>
			{#if mod}<span class="small">{typeName(mod.projectType, true)}</span>{/if}
			<span class="row-actions visible">
				{#if problem.fix}
					<button class="btn" onclick={() => apply(problem.fix!)}>{problem.fix.label}</button>
				{:else if mod}
					<button class="btn" onclick={() => tabs.open('mod', mod.key)}>Show</button>
				{/if}
				<button class="btn quiet" onclick={() => content.hideProblem(problem)}>Hide</button>
			</span>
		</div>
	{:else}
		<div class="trow">No problems</div>
	{/each}
	{#if content.hiddenCount > 0}
		<button class="trow hidden" onclick={() => content.showHiddenProblems()}
			>{content.hiddenCount} hidden · show</button
		>
	{/if}
</div>

<style>
	.severity {
		display: inline-grid;
		place-items: center;
		flex: none;
	}

	.severity.error {
		color: var(--red);
	}

	.severity.warning {
		color: var(--yellow);
	}

	.severity.info {
		color: var(--blue);
	}

	.detail {
		flex: 0 1 auto;
		max-width: 40%;
	}

	.row-actions.visible {
		visibility: visible;
	}

	.hidden {
		width: 100%;
		text-align: left;
	}
</style>

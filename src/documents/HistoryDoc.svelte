<script lang="ts">
	import { untrack } from 'svelte'

	import { layoutGraph, type GraphRow } from '@/lib/git/graph'
	import { commitMenu } from '@/lib/menus'
	import { git } from '@/lib/state/git.svelte'
	import { menus } from '@/lib/state/menus.svelte'
	import { selection } from '@/lib/state/selection.svelte'
	import { tabs, type Doc } from '@/lib/state/tabs.svelte'
	import { ws } from '@/lib/state/workspace.svelte'
	import { graph as loadGraph } from '@/lib/tauri/git'
	import type { GraphCommit } from '@/lib/tauri/types'
	import { ago } from '@/lib/util/format'
	import Empty from '@/ui/Empty.svelte'
	import Segmented from '@/ui/Segmented.svelte'

	interface Props {
		doc: Doc
	}

	let { doc }: Props = $props()

	const PAGE = 200
	const LANE = 14
	const ROW = 28
	const MAX_LANES = 10

	let commits = $state<GraphCommit[]>([])
	let scope = $state<'all' | 'branch'>('all')
	let view = $state<'graph' | 'list'>('graph')
	let loading = $state(false)
	let done = $state(false)
	let generation = 0

	const rows = $derived(layoutGraph(commits))
	const lanes = $derived(Math.min(MAX_LANES, Math.max(1, ...rows.map((r) => r.width))))
	const selected = $derived(selection.of(doc.id)[0] ?? null)

	$effect(() => {
		void scope
		void git.head
		void git.branches
		untrack(reload)
	})

	$effect(() => {
		const reveal = doc.arg
		if (!reveal) return
		untrack(() => {
			selection.set(doc.id, [reveal])
			tabs.setArg(doc.id, '')
		})
	})

	function reload(): void {
		generation++
		commits = []
		done = false
		loading = false
		void more()
	}

	async function more(): Promise<void> {
		const dir = ws.dir
		if (!dir || loading || done) return
		const id = generation
		loading = true
		try {
			const page = await loadGraph(dir, PAGE, commits.length, scope === 'all')
			if (id !== generation) return
			commits = [...commits, ...page]
			done = page.length < PAGE
		} finally {
			if (id === generation) loading = false
		}
	}

	function onscroll(event: Event): void {
		const el = event.currentTarget as HTMLElement
		if (el.scrollTop + el.clientHeight > el.scrollHeight - 400) void more()
	}

	function x(lane: number): number {
		return Math.min(lane, MAX_LANES - 1) * LANE + LANE / 2
	}

	function refLabel(ref: string): { text: string; kind: 'head' | 'tag' | 'remote' | 'branch' } {
		if (ref.startsWith('HEAD -> ')) return { text: ref.slice(8), kind: 'head' }
		if (ref.startsWith('tag: ')) return { text: ref.slice(5), kind: 'tag' }
		if (ref === 'HEAD') return { text: 'HEAD', kind: 'head' }
		return {
			text: ref,
			kind:
				ref.includes('/') && git.remotes.some((r) => ref.startsWith(`${r.name}/`))
					? 'remote'
					: 'branch',
		}
	}

	function select(row: GraphRow): void {
		selection.set(doc.id, [row.commit.hash])
	}
</script>

<div class="bar doc-toolbar">
	<span class="doc-title">History</span>
	<span class="spacer"></span>
	<Segmented
		options={[
			{ value: 'all', label: 'All branches' },
			{ value: 'branch', label: git.branch ?? 'This branch' },
		]}
		bind:value={scope}
	/>
	<Segmented
		options={[
			{ value: 'graph', label: 'Graph' },
			{ value: 'list', label: 'List' },
		]}
		bind:value={view}
	/>
</div>

{#if !loading && commits.length === 0}
	<Empty title="No commits yet" />
{:else}
	<div class="scroll" {onscroll}>
		{#each rows as row (row.commit.hash)}
			<div
				class="trow commit"
				class:selected={selected === row.commit.hash}
				role="option"
				aria-selected={selected === row.commit.hash}
				tabindex="-1"
				onclick={() => select(row)}
				oncontextmenu={(e) => menus.at(e, commitMenu(row.commit.hash))}
			>
				{#if view === 'graph'}
					<svg class="graph" width={lanes * LANE} height={ROW} aria-hidden="true">
						{#each row.through as lane (lane)}
							<line x1={x(lane)} y1="0" x2={x(lane)} y2={ROW} />
						{/each}
						{#if row.fromAbove}
							<line x1={x(row.lane)} y1="0" x2={x(row.lane)} y2={ROW / 2} />
						{/if}
						{#each row.merging as lane (lane)}
							<line x1={x(lane)} y1="0" x2={x(row.lane)} y2={ROW / 2} />
						{/each}
						{#each row.outgoing as lane (lane)}
							<line x1={x(row.lane)} y1={ROW / 2} x2={x(lane)} y2={ROW} />
						{/each}
						<circle
							cx={x(row.lane)}
							cy={ROW / 2}
							r="4"
							class:merge={row.commit.parents.length > 1}
						/>
					</svg>
				{/if}
				<span class="subject ellipsis">{row.commit.subject}</span>
				{#each row.commit.refs as ref (ref)}
					{@const label = refLabel(ref)}
					<span class="tag ref" class:strong={label.kind === 'head'}>{label.text}</span>
				{/each}
				<span class="spacer"></span>
				<span class="author ellipsis">{row.commit.author}</span>
				<span class="when">{ago(row.commit.time * 1000, 'short')}</span>
				<span class="hash mono">{row.commit.short}</span>
			</div>
		{/each}
		{#if loading}<div class="trow"><span class="spinner"></span></div>{/if}
	</div>
{/if}

<style>
	.commit {
		gap: 10px;
		padding-left: 10px;
	}

	.graph {
		flex: none;
		align-self: stretch;
		height: var(--row);
	}

	.graph line {
		stroke: var(--text);
		stroke-width: 1.6;
	}

	.graph circle {
		fill: var(--text);
	}

	.graph circle.merge {
		fill: var(--doc);
		stroke: var(--text);
		stroke-width: 1.6;
	}

	.subject {
		flex: 0 1 auto;
		min-width: 120px;
	}

	.ref {
		font-family: var(--font-mono);
		font-size: 11px;
	}

	.author {
		flex: 0 0 110px;
	}

	.when {
		flex: 0 0 48px;
		text-align: right;
	}

	.hash {
		flex: 0 0 64px;
	}
</style>

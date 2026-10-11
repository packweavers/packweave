<script lang="ts">
	import { parseDiff, sideBySide } from '@/lib/files/diff'

	interface Props {
		diff: string
		split?: boolean
	}

	let { diff, split = false }: Props = $props()

	const lines = $derived(parseDiff(diff))
	const rows = $derived(split ? sideBySide(lines) : [])
</script>

<div class="diff code">
	{#if lines.length === 0}
		<div class="diff-gap">No text changes</div>
	{:else if split}
		{#each rows as row, i (i)}
			<div class="pair">
				{#each [row.left, row.right] as side, s (s)}
					{#if side && side.kind === 'gap'}
						<div class="diff-gap half">⋯</div>
					{:else if side}
						<div
							class="diff-line half"
							class:add={side.kind === 'add'}
							class:del={side.kind === 'del'}
						>
							<span class="num">{(s === 0 ? side.oldNo : side.newNo) ?? ''}</span>
							<span class="sign">{side.kind === 'add' ? '+' : side.kind === 'del' ? '−' : ''}</span>
							<span class="text">{side.text}</span>
						</div>
					{:else}
						<div class="diff-line half blank"></div>
					{/if}
				{/each}
			</div>
		{/each}
	{:else}
		{#each lines as line, i (i)}
			{#if line.kind === 'gap'}
				<div class="diff-gap">⋯</div>
			{:else}
				<div class="diff-line" class:add={line.kind === 'add'} class:del={line.kind === 'del'}>
					<span class="num">{line.oldNo ?? ''}</span>
					<span class="num">{line.newNo ?? ''}</span>
					<span class="sign">{line.kind === 'add' ? '+' : line.kind === 'del' ? '−' : ''}</span>
					<span class="text">{line.text}</span>
				</div>
			{/if}
		{/each}
	{/if}
</div>

<style>
	.diff {
		padding: 6px 0;
		user-select: text;
		-webkit-user-select: text;
	}

	.pair {
		display: flex;
	}

	.half {
		flex: 1 1 50%;
		min-width: 0;
		overflow: hidden;
	}

	.half + .half {
		border-left: 1px solid var(--line);
	}

	.blank {
		background: var(--band);
	}

	.text {
		padding-right: 14px;
	}
</style>

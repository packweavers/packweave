<script lang="ts">
	import Dialog from '@/ui/Dialog.svelte'

	interface Props {
		title: string
		message?: string
		steps?: string[]
		confirm: string
		danger?: boolean
		checkbox?: string
		close: (result?: { checked: boolean }) => void
	}

	let { title, message, steps = [], confirm, danger = false, checkbox, close }: Props = $props()

	let checked = $state(false)
</script>

<Dialog {title} onclose={() => close()}>
	{#if message}<p>{message}</p>{/if}
	{#if steps.length > 0}
		<ol class="steps">
			{#each steps as step, i (step)}
				<li><span class="n mono">{i + 1}</span>{step}</li>
			{/each}
		</ol>
	{/if}
	{#if checkbox}
		<label class="check"><input type="checkbox" bind:checked />{checkbox}</label>
	{/if}
	{#snippet footer()}
		<button class="btn" onclick={() => close()}>Cancel</button>
		<button
			class="btn"
			class:danger-fill={danger}
			class:primary={!danger}
			onclick={() => close({ checked })}
		>
			{confirm}
		</button>
	{/snippet}
</Dialog>

<style>
	.steps {
		margin: 0;
		padding: 0;
		list-style: none;
		display: flex;
		flex-direction: column;
		gap: 6px;
	}

	.steps li {
		display: flex;
		align-items: center;
		gap: 10px;
	}

	.n {
		width: 20px;
		height: 20px;
		display: grid;
		place-items: center;
		background: var(--band);
		font-weight: 600;
		font-size: 11px;
	}
</style>

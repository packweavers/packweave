<script lang="ts">
	import { CircleAlert, X } from '@lucide/svelte'

	import { keysFor } from '@/lib/state/commands.svelte'
	import { toasts } from '@/lib/state/toasts.svelte'

	import Icon from './Icon.svelte'
</script>

<div class="toasts" aria-live="polite">
	{#each toasts.list as toast (toast.id)}
		<div
			class="toast"
			class:error={toast.tone === 'error'}
			role="status"
			onmouseenter={() => toasts.hold(toast.id)}
			onmouseleave={() => toasts.release(toast.id)}
		>
			{#if toast.tone === 'error'}
				<Icon icon={CircleAlert} size={14} />
			{/if}
			<span class="text">{toast.text}</span>
			{#each toast.actions as action (action.label)}
				{@const keys = action.command ? keysFor(action.command) : null}
				<button
					class="btn"
					onclick={() => {
						toasts.dismiss(toast.id)
						action.run()
					}}
				>
					{action.label}
					{#if keys}<span class="keys">{keys}</span>{/if}
				</button>
			{/each}
			<button class="icon-btn" aria-label="Dismiss" onclick={() => toasts.dismiss(toast.id)}>
				<Icon icon={X} size={14} />
			</button>
		</div>
	{/each}
</div>

<style>
	.toasts {
		position: fixed;
		right: 12px;
		bottom: calc(var(--row) + 12px);
		z-index: 55;
		display: flex;
		flex-direction: column;
		align-items: flex-end;
		gap: 8px;
		pointer-events: none;
	}

	.text {
		max-width: 420px;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.error > :global(svg) {
		color: var(--red);
	}
</style>

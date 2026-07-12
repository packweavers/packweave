<script lang="ts">
	import { CircleAlert, CircleCheck, Info, TriangleAlert, X } from '@lucide/svelte'
	import { fly } from 'svelte/transition'
	import { store } from '../lib/store.svelte'

	const accent: Record<string, string> = {
		info: 'border-l-blue',
		success: 'border-l-green',
		warning: 'border-l-orange',
		error: 'border-l-red',
	}
	const iconColor: Record<string, string> = {
		info: 'text-blue',
		success: 'text-green',
		warning: 'text-orange',
		error: 'text-red',
	}
	const icons = {
		info: Info,
		success: CircleCheck,
		warning: TriangleAlert,
		error: CircleAlert,
	} as const

	const iconFor = (kind: string) => icons[kind as keyof typeof icons] ?? Info
</script>

<div class="fixed bottom-4 right-4 z-[90] flex flex-col gap-2 items-end">
	{#each store.toasts as t (t.id)}
		{@const Icon = iconFor(t.kind)}
		<div
			class="flex items-center gap-[0.6rem] bg-bg-super-raised border border-divider border-l-4 {accent[
				t.kind
			] ?? 'border-l-blue'} rounded-md shadow-floating px-3.5 py-2.5 text-sm max-w-[72vw] cursor-default"
			role="presentation"
			transition:fly={{ x: 24, duration: 200 }}
			onclick={() => store.dismiss(t.id)}
		>
			<Icon size={16} class="{iconColor[t.kind] ?? 'text-blue'} shrink-0" />
			<span class="text-body">{t.message}</span>
			<button
				class="bg-transparent border-0 text-secondary cursor-pointer grid place-items-center p-0.5 hover:text-contrast"
				aria-label="Dismiss"
				onclick={(e) => {
					e.stopPropagation()
					store.dismiss(t.id)
				}}
			>
				<X size={14} />
			</button>
		</div>
	{/each}
</div>

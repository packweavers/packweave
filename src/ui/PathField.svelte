<script lang="ts">
	import { onMount } from 'svelte'

	import { tasks } from '@/lib/state/tasks.svelte'

	interface Props {
		value: string
		placeholder?: string
		prefilled?: boolean
		autofocus?: boolean
		suggest: (text: string) => Promise<string[]>
		onsubmit?: (text: string) => void
		onchange?: (text: string) => void
		onblur?: () => void
	}

	let {
		value = $bindable(),
		placeholder = '',
		prefilled = false,
		autofocus = false,
		suggest,
		onsubmit,
		onchange,
		onblur,
	}: Props = $props()

	let input: HTMLInputElement
	let hints = $state<string[]>([])
	let at = $state(-1)
	let open = $state(false)
	let asked = 0

	onMount(() => {
		if (autofocus) input.focus()
	})

	async function refresh(): Promise<void> {
		const ask = ++asked
		const found = await suggest(value).catch(tasks.or('Suggesting paths', [] as string[]))
		if (ask !== asked) return
		hints = found
		at = -1
		open = found.length > 0
	}

	function accept(hint: string): void {
		value = hint
		onchange?.(hint)
		if (hint.endsWith('/')) {
			void refresh()
			return
		}
		open = false
		hints = []
	}

	function onkeydown(event: KeyboardEvent): void {
		if (open && hints.length > 0) {
			if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
				event.preventDefault()
				const step = event.key === 'ArrowDown' ? 1 : -1
				at = (at + step + hints.length) % hints.length
				return
			}
			if (event.key === 'Tab' || (event.key === 'Enter' && at >= 0)) {
				event.preventDefault()
				accept(hints[Math.max(at, 0)])
				return
			}
			if (event.key === 'Escape') {
				event.preventDefault()
				open = false
				return
			}
		}
		if (event.key === 'Enter' && onsubmit) {
			event.preventDefault()
			open = false
			onsubmit(value)
		}
	}
</script>

<div class="path-field">
	<input
		bind:this={input}
		class="field mono"
		class:prefilled
		bind:value
		{placeholder}
		spellcheck="false"
		autocomplete="off"
		oninput={() => {
			onchange?.(value)
			void refresh()
		}}
		onfocus={() => void refresh()}
		onblur={() => {
			open = false
			onblur?.()
		}}
		{onkeydown}
	/>
	{#if open}
		<div class="hints menu">
			{#each hints as hint, i (hint)}
				<button
					class="menu-item mono"
					class:active={i === at}
					onmousedown={(e) => {
						e.preventDefault()
						accept(hint)
					}}>{hint}</button
				>
			{/each}
		</div>
	{/if}
</div>

<style>
	.path-field {
		position: relative;
		flex: 1;
		min-width: 0;
		display: flex;
	}

	.path-field > .field {
		flex: 1;
		min-width: 0;
	}

	.hints {
		position: absolute;
		top: 100%;
		left: 0;
		right: 0;
		max-height: 240px;
	}
</style>

<script lang="ts">
	import { onMount, type Component } from 'svelte'

	import Icon from './Icon.svelte'

	interface Props {
		value: string
		placeholder?: string
		icon?: Component
		error?: string | null
		disabled?: boolean
		mono?: boolean
		prefilled?: boolean
		autofocus?: boolean
		type?: 'text' | 'password' | 'search' | 'url'
		class?: string
		onkeydown?: (event: KeyboardEvent) => void
		oninput?: (value: string) => void
		onblur?: () => void
	}

	let {
		value = $bindable(),
		placeholder = '',
		icon,
		error = null,
		disabled = false,
		mono = false,
		prefilled = false,
		autofocus = false,
		type = 'text',
		class: extra = '',
		onkeydown,
		oninput,
		onblur,
	}: Props = $props()

	let input: HTMLInputElement

	onMount(() => {
		if (autofocus) input.focus()
	})

	export function focus(): void {
		input.focus()
		input.select()
	}
</script>

<label class="field {extra}" class:error={!!error} class:disabled>
	{#if icon}
		<Icon {icon} size={14} />
	{/if}
	<input
		bind:this={input}
		bind:value
		class:mono
		class:prefilled
		{type}
		{placeholder}
		{disabled}
		spellcheck="false"
		autocomplete="off"
		{onkeydown}
		oninput={() => oninput?.(value)}
		{onblur}
	/>
</label>
{#if error}
	<span class="field-error">{error}</span>
{/if}

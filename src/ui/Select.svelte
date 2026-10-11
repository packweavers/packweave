<script lang="ts" generics="T extends string">
	import { ChevronDown } from '@lucide/svelte'

	import { menus } from '@/lib/state/menus.svelte'

	import Icon from './Icon.svelte'

	interface Option {
		value: T
		label: string
	}

	interface Props {
		options: Option[]
		value: T
		placeholder?: string
		disabled?: boolean
		mono?: boolean
		class?: string
		onchange?: (value: T) => void
	}

	let {
		options,
		value = $bindable(),
		placeholder = 'Choose…',
		disabled = false,
		mono = false,
		class: extra = '',
		onchange,
	}: Props = $props()

	const current = $derived(options.find((o) => o.value === value))

	function open(event: MouseEvent): void {
		menus.below(
			event.currentTarget as Element,
			options.map((option) => ({
				label: option.label,
				selected: option.value === value,
				run: () => {
					value = option.value
					onchange?.(option.value)
				},
			})),
		)
	}
</script>

<button class="field select {extra}" class:disabled {disabled} onclick={open}>
	<span class="ellipsis" class:mono class:placeholder={!current}
		>{current?.label ?? placeholder}</span
	>
	<Icon icon={ChevronDown} size={14} />
</button>

<style>
	.placeholder {
		opacity: 0.5;
		font-style: italic;
	}

	.select {
		justify-content: space-between;
		text-align: left;
	}
</style>

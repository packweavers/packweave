<script lang="ts" generics="T extends string">
	interface Option {
		value: T
		label: string
	}

	interface Props {
		options: Option[]
		value: T
		small?: boolean
		class?: string
		onchange?: (value: T) => void
	}

	let { options, value = $bindable(), small = false, class: extra = '', onchange }: Props = $props()

	function pick(next: T): void {
		value = next
		onchange?.(next)
	}
</script>

<div class="seg {extra}" class:small role="radiogroup">
	{#each options as option (option.value)}
		<button
			role="radio"
			aria-checked={option.value === value}
			class:on={option.value === value}
			onclick={() => pick(option.value)}
		>
			{option.label}
		</button>
	{/each}
</div>

<script lang="ts">
	import type { Snippet } from 'svelte'

	type Color = 'standard' | 'brand' | 'red' | 'orange' | 'green' | 'blue' | 'purple'
	type BType = 'standard' | 'transparent' | 'outlined' | 'highlight'
	type Size = 'standard' | 'small' | 'large'

	let {
		color = 'standard',
		type = 'standard',
		size = 'standard',
		circular = false,
		disabled = false,
		children,
		...rest
	}: {
		color?: Color
		type?: BType
		size?: Size
		circular?: boolean
		disabled?: boolean
		children?: Snippet
		[key: string]: unknown
	} = $props()

	const sizeCls: Record<Size, string> = {
		standard: 'h-9 px-3.5 text-sm',
		small: 'h-[1.85rem] px-2.5 text-[0.8rem] rounded-sm',
		large: 'h-[2.7rem] px-5 text-[0.95rem]',
	}
	const circCls: Record<Size, string> = {
		standard: 'h-9 w-9 px-0',
		small: 'h-[1.85rem] w-[1.85rem] px-0 rounded-sm',
		large: 'h-[2.7rem] w-[2.7rem] px-0',
	}
	const text: Record<Color, string> = {
		standard: 'text-secondary',
		brand: 'text-brand',
		red: 'text-red',
		orange: 'text-orange',
		green: 'text-green',
		blue: 'text-blue',
		purple: 'text-purple',
	}
	const filled: Record<Color, string> = {
		standard: 'bg-button-bg text-contrast hover:bg-button-bg-hover',
		brand: 'bg-brand text-on-brand hover:bg-brand-hover',
		red: 'bg-red text-white hover:brightness-110',
		orange: 'bg-orange text-white hover:brightness-110',
		green: 'bg-green text-white hover:brightness-110',
		blue: 'bg-blue text-on-brand hover:brightness-110',
		purple: 'bg-purple text-white hover:brightness-110',
	}

	function variant(c: Color, t: BType): string {
		if (t === 'standard') return filled[c]
		if (t === 'transparent')
			return c === 'standard'
				? 'bg-transparent text-secondary hover:bg-button-bg hover:text-contrast'
				: `bg-transparent ${text[c]} hover:bg-button-bg`
		if (t === 'outlined')
			return c === 'standard'
				? 'border-divider text-body hover:bg-button-bg hover:text-contrast'
				: `border-divider ${text[c]} hover:bg-button-bg`
		return c === 'standard' ? 'bg-button-bg text-contrast' : `bg-button-bg ${text[c]}`
	}
</script>

<button
	type="button"
	{disabled}
	class="inline-flex items-center justify-center gap-1.5 font-semibold leading-none whitespace-nowrap cursor-pointer border border-transparent rounded-md transition active:scale-[0.97] disabled:opacity-45 disabled:cursor-default {circular
		? circCls[size]
		: sizeCls[size]} {variant(color, type)}"
	{...rest}
>
	{@render children?.()}
</button>

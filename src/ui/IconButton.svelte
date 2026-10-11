<script lang="ts">
	import type { Component } from 'svelte'

	import Icon from './Icon.svelte'
	import { tooltip } from './tooltip'

	interface Props {
		icon: Component
		label: string
		command?: string
		active?: boolean
		disabled?: boolean
		small?: boolean
		size?: 12 | 14 | 16 | 20
		badge?: number
		badgeTone?: 'warn' | 'error' | null
		class?: string
		onclick?: (event: MouseEvent) => void
		oncontextmenu?: (event: MouseEvent) => void
	}

	let {
		icon,
		label,
		command,
		active = false,
		disabled = false,
		small = false,
		size = 16,
		badge = 0,
		badgeTone = null,
		class: extra = '',
		onclick,
		oncontextmenu,
	}: Props = $props()
</script>

<button
	class="icon-btn {extra}"
	class:on={active}
	class:small
	aria-label={label}
	aria-pressed={active}
	{disabled}
	{onclick}
	{oncontextmenu}
	use:tooltip={{ text: label, command }}
>
	<Icon {icon} {size} />
	{#if badge > 0}
		<span class="count {badgeTone ?? ''}">{badge > 99 ? '99+' : badge}</span>
	{/if}
</button>

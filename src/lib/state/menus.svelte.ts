import type { Component } from 'svelte'

export interface MenuItem {
	label: string
	icon?: Component
	command?: string
	danger?: boolean
	disabled?: boolean
	selected?: boolean
	run: () => void
}

export type MenuEntry = MenuItem | 'separator' | { heading: string }

export interface OpenMenu {
	x: number
	y: number
	entries: MenuEntry[]
}

class Menus {
	current = $state.raw<OpenMenu | null>(null)

	open(x: number, y: number, entries: MenuEntry[]): void {
		const visible = entries.filter(
			(e, i, all) => e !== 'separator' || (i > 0 && all[i - 1] !== 'separator'),
		)
		this.current = visible.length ? { x, y, entries: visible } : null
	}

	at(event: MouseEvent, entries: MenuEntry[]): void {
		event.preventDefault()
		event.stopPropagation()
		this.open(event.clientX, event.clientY, entries)
	}

	below(element: Element, entries: MenuEntry[]): void {
		const box = element.getBoundingClientRect()
		this.open(box.left, box.bottom, entries)
	}

	close(): void {
		this.current = null
	}
}

export const menus = new Menus()

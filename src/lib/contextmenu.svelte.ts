import type { Component } from 'svelte'

export interface MenuItem {
	label?: string
	icon?: Component
	danger?: boolean
	disabled?: boolean
	separator?: boolean
	onSelect?: () => void
}

type Entry = MenuItem | null | false | undefined

export const ctxMenu = $state({
	open: false,
	x: 0,
	y: 0,
	items: [] as MenuItem[],
})

export function openContextMenu(e: MouseEvent, items: Entry[]) {
	const filtered = items.filter((i): i is MenuItem => !!i)
	if (!filtered.length) return
	e.preventDefault()
	e.stopPropagation()
	ctxMenu.items = filtered
	ctxMenu.x = e.clientX
	ctxMenu.y = e.clientY
	ctxMenu.open = true
}

export function closeContextMenu() {
	ctxMenu.open = false
}

export function contextMenu(node: HTMLElement, get: () => Entry[]) {
	let current = get
	const handler = (e: MouseEvent) => openContextMenu(e, current())
	node.addEventListener('contextmenu', handler)
	return {
		update(g: () => Entry[]) {
			current = g
		},
		destroy() {
			node.removeEventListener('contextmenu', handler)
		},
	}
}

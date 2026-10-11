import { computePosition, flip, offset, shift } from '@floating-ui/dom'

import { keysFor } from '@/lib/state/commands.svelte'

export interface TooltipOptions {
	text: string
	command?: string
}

const DELAY = 500
let element: HTMLDivElement | null = null
let timer: ReturnType<typeof setTimeout> | null = null

function tooltipElement(): HTMLDivElement {
	if (!element) {
		element = document.createElement('div')
		element.className = 'tooltip'
		element.setAttribute('role', 'tooltip')
		element.style.display = 'none'
		document.body.appendChild(element)
	}
	return element
}

function hide(): void {
	if (timer) clearTimeout(timer)
	timer = null
	if (element) element.style.display = 'none'
}

async function show(anchor: HTMLElement, options: TooltipOptions): Promise<void> {
	const tip = tooltipElement()
	tip.replaceChildren(document.createTextNode(options.text))
	const keys = options.command ? keysFor(options.command) : null
	if (keys) {
		const kbd = document.createElement('span')
		kbd.className = 'kbd'
		kbd.textContent = keys
		tip.appendChild(kbd)
	}
	tip.style.display = 'inline-flex'
	const { x, y } = await computePosition(anchor, tip, {
		placement: 'bottom',
		strategy: 'fixed',
		middleware: [offset(6), flip(), shift({ padding: 6 })],
	})
	tip.style.left = `${x}px`
	tip.style.top = `${y}px`
}

export function tooltip(node: HTMLElement, options: TooltipOptions | string) {
	let current: TooltipOptions = typeof options === 'string' ? { text: options } : options
	const enter = () => {
		if (timer) clearTimeout(timer)
		timer = setTimeout(() => void show(node, current), DELAY)
	}
	node.addEventListener('mouseenter', enter)
	node.addEventListener('mouseleave', hide)
	node.addEventListener('mousedown', hide)
	return {
		update(next: TooltipOptions | string) {
			current = typeof next === 'string' ? { text: next } : next
		},
		destroy() {
			node.removeEventListener('mouseenter', enter)
			node.removeEventListener('mouseleave', hide)
			node.removeEventListener('mousedown', hide)
			hide()
		},
	}
}

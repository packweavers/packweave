import { computePosition, flip, shift, offset } from '@floating-ui/dom'

export function tooltip(node: HTMLElement, text: string) {
	let current = text
	let tip: HTMLDivElement | null = null

	function show() {
		if (!current) return
		tip = document.createElement('div')
		tip.textContent = current
		tip.style.cssText =
			'position:fixed;left:0;top:0;z-index:80;pointer-events:none;background:var(--color-tooltip-bg);color:var(--color-tooltip-text);padding:0.35rem 0.55rem;border-radius:var(--radius-md);font-size:0.78rem;font-weight:500;box-shadow:var(--shadow-floating);max-width:20rem'
		document.body.appendChild(tip)
		computePosition(node, tip, {
			placement: 'top',
			strategy: 'fixed',
			middleware: [offset(6), flip(), shift({ padding: 6 })],
		}).then(({ x, y }) => {
			if (tip) {
				tip.style.left = `${x}px`
				tip.style.top = `${y}px`
			}
		})
	}
	function hide() {
		tip?.remove()
		tip = null
	}

	node.addEventListener('mouseenter', show)
	node.addEventListener('mouseleave', hide)
	node.addEventListener('click', hide)

	return {
		update(t: string) {
			current = t
			if (tip) tip.textContent = t
		},
		destroy() {
			hide()
			node.removeEventListener('mouseenter', show)
			node.removeEventListener('mouseleave', hide)
			node.removeEventListener('click', hide)
		},
	}
}

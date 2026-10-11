import { isPanelTab } from './layout.svelte'
import type { Doc } from './tabs.svelte'

const SNAP = 40

export interface DropSpot {
	group: number
	index: number
}

export function spotAt(x: number, y: number): DropSpot | null {
	for (const strip of document.querySelectorAll<HTMLElement>('[data-strip]')) {
		const box = strip.getBoundingClientRect()
		if (x < box.left || x > box.right || y < box.top - SNAP || y > box.bottom + SNAP) continue
		const tabEls = [...strip.querySelectorAll<HTMLElement>('[data-tab]')]
		const after = tabEls.findIndex((el) => {
			const tab = el.getBoundingClientRect()
			return x < tab.left + tab.width / 2
		})
		return { group: Number(strip.dataset.strip), index: after < 0 ? tabEls.length : after }
	}
	return null
}

function overPanelBar(x: number, y: number): boolean {
	const bar = document.querySelector<HTMLElement>('[data-panel-bar]')
	if (!bar) return false
	const box = bar.getBoundingClientRect()
	return x >= box.left && x <= box.right && y >= box.top - SNAP / 2 && y <= box.bottom + SNAP / 2
}

class TabDrag {
	doc = $state.raw<Doc | null>(null)
	x = $state(0)
	y = $state(0)
	spot = $state.raw<DropSpot | null>(null)
	inside = $state(true)
	panel = $state(false)

	begin(doc: Doc): void {
		this.doc = doc
	}

	move(x: number, y: number): void {
		this.x = x
		this.y = y
		this.inside = x >= 0 && y >= 0 && x <= window.innerWidth && y <= window.innerHeight
		this.panel = this.inside && this.doc !== null && isPanelTab(this.doc.kind) && overPanelBar(x, y)
		this.spot = this.inside && !this.panel ? spotAt(x, y) : null
	}

	end(): void {
		this.doc = null
		this.spot = null
		this.inside = true
		this.panel = false
	}
}

export const tabDrag = new TabDrag()

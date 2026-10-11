import { toasts } from '@/lib/state/toasts.svelte'

interface Running {
	scope: string
	label: string
	index: number
	mount: Element | null
	button: HTMLElement
}

const running = new Set<Running>()
let observer: MutationObserver | null = null

function labelOf(button: Element): string {
	return (
		button.textContent?.trim() ||
		button.getAttribute('aria-label') ||
		button.getAttribute('title') ||
		''
	)
}

function matching(scope: Element, label: string): HTMLElement[] {
	return [...scope.querySelectorAll<HTMLElement>('button')].filter(
		(b) => b.closest('[data-busy-scope]') === scope && labelOf(b) === label,
	)
}

function entryFor(button: HTMLElement): Running | null {
	const scope = button.closest('[data-busy-scope]')
	const id = scope?.getAttribute('data-busy-scope')
	if (!scope || !id) return null
	const label = labelOf(button)
	return {
		scope: id,
		label,
		index: matching(scope, label).indexOf(button),
		mount: button.closest('[data-busy-scope] > *'),
		button,
	}
}

function restore(): void {
	for (const entry of running) {
		if (entry.mount?.isConnected) continue
		const scope = [...document.querySelectorAll('[data-busy-scope]')].find(
			(el) => el.getAttribute('data-busy-scope') === entry.scope,
		)
		const button = scope ? matching(scope, entry.label)[entry.index] : undefined
		if (!button) continue
		button.setAttribute('aria-busy', 'true')
		entry.button = button
		entry.mount = button.closest('[data-busy-scope] > *')
	}
}

export function pending(work: () => Promise<unknown>): (event: MouseEvent) => void {
	return (event) => {
		const button = event.currentTarget
		if (!(button instanceof HTMLElement) || button.getAttribute('aria-busy') === 'true') return
		const entry = entryFor(button)
		if (entry) {
			for (const other of running) {
				if (
					other.scope === entry.scope &&
					other.label === entry.label &&
					other.index === entry.index
				) {
					return
				}
			}
			running.add(entry)
			observer ??= new MutationObserver(restore)
			observer.observe(document.body, { childList: true, subtree: true })
		}
		button.setAttribute('aria-busy', 'true')
		void work()
			.catch((error) => toasts.error(error))
			.finally(() => {
				button.removeAttribute('aria-busy')
				if (!entry) return
				entry.button.removeAttribute('aria-busy')
				running.delete(entry)
				if (running.size === 0) observer?.disconnect()
			})
	}
}

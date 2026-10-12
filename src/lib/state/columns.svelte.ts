import { tasks } from './tasks.svelte'

export interface Column {
	id: string
	label: string
	min: number
	width?: number
	max?: number
	hide?: number
}

export interface TableSpacing {
	gap?: number
	left?: number
	right?: number
}

const PREFIX = 'columns:'
const MAX_WIDTH = 640

function readWidths(id: string): Record<string, number> {
	try {
		const saved = JSON.parse(localStorage.getItem(PREFIX + id) ?? '{}') as Record<string, unknown>
		return Object.fromEntries(
			Object.entries(saved).filter(
				(entry): entry is [string, number] =>
					typeof entry[1] === 'number' && Number.isFinite(entry[1]),
			),
		)
	} catch (error) {
		tasks.fail('Reading the saved column widths', error)
		return {}
	}
}

export class Table {
	readonly id: string
	readonly columns: Column[]
	readonly gap: number
	readonly left: number
	readonly right: number
	available = $state(0)
	#widths = $state<Record<string, number>>({})

	constructor(id: string, columns: Column[], spacing: TableSpacing = {}) {
		this.id = id
		this.columns = columns
		this.gap = spacing.gap ?? 12
		this.left = spacing.left ?? 16
		this.right = spacing.right ?? 12
		this.#widths = readWidths(id)
	}

	width(column: Column): number {
		const value = this.#widths[column.id] ?? column.width ?? column.min
		return Math.min(column.max ?? MAX_WIDTH, Math.max(column.min, value))
	}

	#needed(columns: Column[]): number {
		const cells = columns.reduce(
			(sum, c) => sum + (c.width === undefined ? c.min : this.width(c)),
			0,
		)
		return cells + this.gap * (columns.length - 1) + this.left + this.right
	}

	shown = $derived.by(() => {
		let shown = this.columns
		if (this.available <= 0) return shown
		const hideable = this.columns
			.filter((c) => c.hide !== undefined)
			.sort((a, b) => (b.hide ?? 0) - (a.hide ?? 0))
		for (const column of hideable) {
			if (this.#needed(shown) <= this.available) break
			shown = shown.filter((c) => c !== column)
		}
		return shown
	})

	#visible = $derived(new Set(this.shown.map((c) => c.id)))

	template = $derived(
		this.shown
			.map((c) => (c.width === undefined ? `minmax(${c.min}px, 1fr)` : `${this.width(c)}px`))
			.join(' '),
	)

	style = $derived.by(
		() =>
			`display: grid; grid-template-columns: ${this.template}; column-gap: ${this.gap}px; padding-left: ${this.left}px; padding-right: ${this.right}px`,
	)

	has(id: string): boolean {
		return this.#visible.has(id)
	}

	resize(column: Column, width: number): void {
		this.#widths[column.id] = width
	}

	reset(column: Column): void {
		delete this.#widths[column.id]
		this.persist()
	}

	persist(): void {
		try {
			localStorage.setItem(PREFIX + this.id, JSON.stringify(this.#widths))
		} catch (error) {
			tasks.fail('Saving the column widths', error)
		}
	}

	measure = (node: HTMLElement): (() => void) => {
		const observer = new ResizeObserver(() => {
			this.available = node.clientWidth
		})
		observer.observe(node)
		this.available = node.clientWidth
		return () => observer.disconnect()
	}
}

export interface ListSelection {
	selected: string[]
	anchor: string | null
	cursor: string | null
}

const EMPTY_SELECTION: ListSelection = { selected: [], anchor: null, cursor: null }

function range(ids: string[], from: string, to: string): string[] {
	const a = ids.indexOf(from)
	const b = ids.indexOf(to)
	if (a < 0 || b < 0) return [to]
	return ids.slice(Math.min(a, b), Math.max(a, b) + 1)
}

export function clickSelect(
	state: ListSelection,
	ids: string[],
	id: string,
	event: MouseEvent,
): ListSelection {
	if (event.shiftKey && state.anchor) {
		return { selected: range(ids, state.anchor, id), anchor: state.anchor, cursor: id }
	}
	if (event.metaKey || event.ctrlKey) {
		const selected = state.selected.includes(id)
			? state.selected.filter((s) => s !== id)
			: [...state.selected, id]
		return { selected, anchor: id, cursor: id }
	}
	return { selected: [id], anchor: id, cursor: id }
}

export function keySelect(
	state: ListSelection,
	ids: string[],
	event: KeyboardEvent,
): ListSelection | null {
	if (ids.length === 0) return null
	if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === 'a') {
		return { selected: [...ids], anchor: ids[0], cursor: ids[ids.length - 1] }
	}
	if (event.key === 'Escape' && state.selected.length > 0) return EMPTY_SELECTION
	const step = event.key === 'ArrowDown' ? 1 : event.key === 'ArrowUp' ? -1 : 0
	if (step === 0) return null
	const at = state.cursor ? ids.indexOf(state.cursor) : -1
	const next = ids[Math.min(ids.length - 1, Math.max(0, at < 0 ? 0 : at + step))]
	if (event.shiftKey && state.anchor) {
		return { selected: range(ids, state.anchor, next), anchor: state.anchor, cursor: next }
	}
	return { selected: [next], anchor: next, cursor: next }
}

export function focusOnMount(node: HTMLElement, select = false) {
	queueMicrotask(() => {
		node.focus()
		if (select && node instanceof HTMLInputElement) node.select()
	})
}

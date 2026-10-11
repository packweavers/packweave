export type ContentChange = 'added' | 'changed' | 'removed'

export function contentChange(kind: string): ContentChange {
	switch (kind) {
		case 'added':
		case 'enabled':
			return 'added'
		case 'removed':
		case 'disabled':
			return 'removed'
		default:
			return 'changed'
	}
}

import type { FileKind } from '@/lib/files/kind'

import { tasks } from './tasks.svelte'

export type FileMode = 'form' | 'text' | 'compare' | 'edit' | 'preview' | 'split'

const KEY = 'file-modes'

function read(): Partial<Record<FileKind, FileMode>> {
	try {
		return JSON.parse(localStorage.getItem(KEY) ?? '{}') as Partial<Record<FileKind, FileMode>>
	} catch (error) {
		tasks.fail('Reading the saved file views', error)
		return {}
	}
}

class FileView {
	modes = $state<Partial<Record<FileKind, FileMode>>>(read())

	modeFor(kind: FileKind, fallback: FileMode): FileMode {
		return this.modes[kind] ?? fallback
	}

	remember(kind: FileKind, mode: FileMode): void {
		if (mode === 'compare') return
		this.modes[kind] = mode
		try {
			localStorage.setItem(KEY, JSON.stringify(this.modes))
		} catch (error) {
			tasks.fail('Saving the file views', error)
		}
	}
}

export const fileView = new FileView()

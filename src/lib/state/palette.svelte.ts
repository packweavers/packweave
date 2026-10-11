import type { PaletteMode } from '@/lib/palette'

class PaletteState {
	open = $state(false)
	query = $state('')
	base = $state<PaletteMode>('all')
	preferWindow = $state(false)

	show(prefix = '', base: PaletteMode = 'all', preferWindow = false): void {
		this.query = prefix
		this.base = base
		this.preferWindow = preferWindow
		this.open = true
	}

	hide(): void {
		this.open = false
	}
}

export const palette = new PaletteState()

import { getCurrentWebview } from '@tauri-apps/api/webview'

import { app } from './app.svelte'
import { codeLine } from './prefs'
import { tasks } from './tasks.svelte'

export function followTheme(): void {
	$effect.root(() => {
		$effect(() => {
			document.documentElement.dataset.theme = app.theme
		})
	})
}

export function followScale(): void {
	$effect.root(() => {
		$effect(() => {
			const scale = app.prefs.uiScale
			const root = document.documentElement.style
			root.setProperty('--ui-zoom', String(scale))
			void getCurrentWebview()
				.setZoom(scale)
				.catch((error) => tasks.fail('Changing the UI scale', error))
		})
		$effect(() => {
			const size = app.prefs.editorSize
			const root = document.documentElement.style
			root.setProperty('--code-size', `${size}px`)
			root.setProperty('--code-line', `${codeLine(size)}px`)
		})
	})
}

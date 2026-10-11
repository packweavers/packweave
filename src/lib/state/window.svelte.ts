import { getCurrentWindow } from '@tauri-apps/api/window'

class WindowState {
	maximized = $state(false)
	fullscreen = $state(false)

	follow(): () => void {
		const win = getCurrentWindow()
		const refresh = async (): Promise<void> => {
			const [maximized, fullscreen] = await Promise.all([win.isMaximized(), win.isFullscreen()])
			this.maximized = maximized
			this.fullscreen = fullscreen
		}
		void refresh()
		const stop = win.onResized(() => void refresh())
		return () => void stop.then((unlisten) => unlisten())
	}
}

export const windowState = new WindowState()

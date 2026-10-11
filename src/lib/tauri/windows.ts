import { getCurrentWindow } from '@tauri-apps/api/window'
import { open as openDialog } from '@tauri-apps/plugin-dialog'
import { openPath, openUrl, revealItemInDir } from '@tauri-apps/plugin-opener'

import { toasts } from '@/lib/state/toasts.svelte'

import { call } from './call'
import type { DropTarget, OpenWindow, TabPayload, WindowPack } from './types'

export function windowPack(): Promise<WindowPack | null> {
	return call('window_pack')
}

export function attachWindow(dir: string | null, title: string): Promise<string | null> {
	return call('window_attach', { dir, title })
}

export function setWindowTitle(title: string): Promise<void> {
	return getCurrentWindow().setTitle(title)
}

export function openPackWindow(dir: string, title: string): Promise<string> {
	return call('open_pack_window', { dir, title })
}

export function detachTab(tab: TabPayload, x: number | null, y: number | null): Promise<string> {
	return call('detach_tab', { tab, x, y })
}

export function showDoc(label: string, id: string): Promise<void> {
	return call('show_doc', { label, id })
}

export function tabDropTarget(x: number, y: number): Promise<DropTarget | null> {
	return call('tab_drop_target', { x, y })
}

export function sendTab(
	label: string,
	tab: TabPayload,
	x: number | null,
	y: number | null,
): Promise<void> {
	return call('send_tab', { label, tab, x, y })
}

export function mainWindow(): Promise<string | null> {
	return call('main_window')
}

export function openSettingsWindow(page: string | null = null): Promise<void> {
	return call('open_settings_window', { page })
}

export function packWindows(): Promise<OpenWindow[]> {
	return call('pack_windows')
}

export function closePackWindow(dir: string): Promise<void> {
	return call('close_pack_window', { dir })
}

export async function pickFolder(title: string): Promise<string | null> {
	const picked = await openDialog({ directory: true, multiple: false, title })
	return typeof picked === 'string' ? picked : null
}

export async function pickFile(
	title: string,
	filters: { name: string; extensions: string[] }[] = [],
): Promise<string | null> {
	const picked = await openDialog({ directory: false, multiple: false, title, filters })
	return typeof picked === 'string' ? picked : null
}

export function openExternal(url: string): Promise<void> {
	return openUrl(url).catch((error) => {
		toasts.error(error)
	})
}

export function openFolder(path: string): Promise<void> {
	return openPath(path).catch((error) => {
		toasts.error(error)
	})
}

export function openInDefaultApp(path: string): Promise<void> {
	return openPath(path).catch((error) => {
		toasts.error(error)
	})
}

export function reveal(path: string): Promise<void> {
	return revealItemInDir(path).catch((error) => {
		toasts.error(error)
	})
}

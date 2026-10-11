import ConfirmDialog from '@/dialogs/ConfirmDialog.svelte'
import { app } from '@/lib/state/app.svelte'
import { dialogs } from '@/lib/state/dialogs.svelte'
import { files } from '@/lib/state/files.svelte'
import { isPanelTab, layout, type PanelTab } from '@/lib/state/layout.svelte'
import { panels } from '@/lib/state/panels.svelte'
import { spotAt } from '@/lib/state/tabdrag.svelte'
import { bareDoc, docId, isDocKind, tabs, type Doc } from '@/lib/state/tabs.svelte'
import { toasts } from '@/lib/state/toasts.svelte'
import { ws } from '@/lib/state/workspace.svelte'
import type { DropTarget, TabPayload } from '@/lib/tauri/types'
import { detachTab, mainWindow, sendTab, tabDropTarget } from '@/lib/tauri/windows'
import { plural } from '@/lib/util/format'
import { basename } from '@/lib/util/path'

function groupOf(doc: Doc): Doc[] {
	return tabs.groups.find((g) => g.docs.some((d) => d.id === doc.id))?.docs ?? []
}

async function closeDocs(docs: Doc[]): Promise<boolean> {
	const closing = docs.filter((d) => !d.pinned)
	const unsaved = closing.filter((d) => d.kind === 'file' && files.isDirty(d.arg))
	if (unsaved.length > 0) {
		const ok = await dialogs.open(ConfirmDialog, {
			title:
				unsaved.length === 1
					? `Close ${basename(unsaved[0].arg)} without saving?`
					: `Close ${plural(unsaved.length, 'unsaved file', 'unsaved files')}?`,
			message: 'Unsaved edits are lost.',
			confirm: 'Close without saving',
			danger: true,
		})
		if (!ok) return false
		for (const doc of unsaved) files.forget(doc.arg)
	}
	for (const doc of closing) tabs.close(doc.id)
	return true
}

export async function closeDoc(doc: Doc): Promise<void> {
	await closeDocs([doc])
}

export async function closeOthers(doc: Doc): Promise<void> {
	if (await closeDocs(groupOf(doc).filter((d) => d.id !== doc.id))) tabs.activate(doc.id)
}

export async function closeToRight(doc: Doc): Promise<void> {
	const docs = groupOf(doc)
	await closeDocs(docs.slice(docs.findIndex((d) => d.id === doc.id) + 1))
}

function payloadOf(doc: Doc): TabPayload {
	const buffer = doc.kind === 'file' && files.isDirty(doc.arg) ? files.buffers[doc.arg] : null
	return {
		kind: doc.kind,
		arg: doc.arg,
		pinned: doc.pinned,
		buffer: buffer ? { original: buffer.original, text: buffer.text } : null,
	}
}

async function handOff(doc: Doc, send: (tab: TabPayload) => Promise<string>): Promise<void> {
	let label: string
	try {
		label = await send(payloadOf(doc))
	} catch (error) {
		toasts.error(error)
		return
	}
	if (isPanelTab(doc.kind)) panels.movedTo(label, doc.kind)
	tabs.detach(doc.id)
	if (doc.kind === 'file') files.forget(doc.arg)
}

export async function dropOutside(doc: Doc, x: number, y: number): Promise<void> {
	let target: DropTarget | null = null
	try {
		target = await tabDropTarget(x, y)
	} catch (error) {
		toasts.error(error)
		return
	}
	if (target && !target.sameBranch) {
		toasts.info('Tabs only move between windows of the same branch')
		return
	}
	const label = target?.label
	await handOff(doc, (tab) =>
		label ? sendTab(label, tab, x, y).then(() => label) : detachTab(tab, x, y),
	)
}

export async function moveToNewWindow(doc: Doc): Promise<void> {
	await handOff(doc, (tab) => detachTab(tab, null, null))
}

export async function moveToMainWindow(doc: Doc): Promise<void> {
	const label = await mainWindow().catch(() => null)
	if (!label) {
		toasts.info('The main window is closed')
		return
	}
	await handOff(doc, (tab) => sendTab(label, tab, null, null).then(() => label))
}

export function moveToPanel(doc: Doc): void {
	if (!isPanelTab(doc.kind)) return
	tabs.detach(doc.id)
	layout.openPanel(doc.kind)
}

export function movePanelToEditor(tab: PanelTab): void {
	tabs.open(tab)
}

export async function movePanelToNewWindow(tab: PanelTab): Promise<void> {
	await moveToNewWindow(bareDoc(tab))
}

export async function dropPanelOutside(tab: PanelTab, x: number, y: number): Promise<void> {
	await dropOutside(bareDoc(tab), x, y)
}

export function receiveTab(tab: TabPayload, x: number | null, y: number | null): void {
	if (!isDocKind(tab.kind)) return
	const zoom = app.prefs.uiScale
	const spot =
		x === null || y === null
			? null
			: spotAt((x - window.screenX) / zoom, (y - window.screenY) / zoom)
	if (isPanelTab(tab.kind) && !ws.detached && !spot) {
		layout.openPanel(tab.kind)
		return
	}
	ws.receiveTab(tab)
	if (spot) tabs.moveTo(docId(tab.kind, tab.arg), spot.group, spot.index)
}

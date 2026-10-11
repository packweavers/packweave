import { emit, listen } from '@tauri-apps/api/event'
import { getCurrentWindow } from '@tauri-apps/api/window'

import { showDoc } from '@/lib/tauri/windows'
import { sameDir } from '@/lib/util/path'

import { layout, PANEL_TABS, type PanelTab } from './layout.svelte'
import { tabs } from './tabs.svelte'
import { ws } from './workspace.svelte'

const DOCS_EVENT = 'panel-docs'
const ASK_EVENT = 'panel-docs-ask'
const GONE_EVENT = 'window-closed'

interface PanelDocs {
	label: string
	dir: string
	kinds: PanelTab[]
}

interface Holder {
	dir: string
	kinds: PanelTab[]
}

class Panels {
	elsewhere = $state<Record<string, Holder>>({})
	#sent = ''

	get here(): PanelTab[] {
		return PANEL_TABS.filter((tab) => tabs.isOpen(tab))
	}

	get shown(): PanelTab[] {
		return PANEL_TABS.filter((tab) => !this.away(tab))
	}

	holder(tab: PanelTab): string | null {
		const dir = ws.dir
		if (!dir) return null
		for (const [label, held] of Object.entries(this.elsewhere)) {
			if (held.kinds.includes(tab) && sameDir(held.dir, dir)) return label
		}
		return null
	}

	away(tab: PanelTab): boolean {
		return tabs.isOpen(tab) || this.holder(tab) !== null
	}

	open(tab: PanelTab): void {
		if (tabs.isOpen(tab)) {
			tabs.activate(tab)
			return
		}
		const label = this.holder(tab)
		if (label) {
			void showDoc(label, tab).catch(() => this.#drop(label))
			return
		}
		layout.openPanel(tab)
	}

	movedTo(label: string, tab: PanelTab): void {
		if (!ws.dir) return
		const held = this.elsewhere[label]
		const kinds = held ? [...new Set([...held.kinds, tab])] : [tab]
		this.elsewhere = { ...this.elsewhere, [label]: { dir: ws.dir, kinds } }
	}

	follow(): () => void {
		const me = getCurrentWindow().label
		const stops = [
			listen<PanelDocs>(DOCS_EVENT, ({ payload }) => {
				if (payload.label === me) return
				if (payload.kinds.length === 0) {
					this.#drop(payload.label)
					return
				}
				this.elsewhere = {
					...this.elsewhere,
					[payload.label]: { dir: payload.dir, kinds: payload.kinds },
				}
			}),
			listen(ASK_EVENT, () => {
				this.#sent = ''
				this.announce()
			}),
			listen<string>(GONE_EVENT, ({ payload }) => this.#drop(payload)),
		]
		void emit(ASK_EVENT)
		return () => {
			for (const stop of stops) void stop.then((unlisten) => unlisten())
		}
	}

	announce(): void {
		const payload: PanelDocs = {
			label: getCurrentWindow().label,
			dir: ws.dir ?? '',
			kinds: this.here,
		}
		const key = `${payload.dir}|${payload.kinds.join(',')}`
		if (key === this.#sent) return
		this.#sent = key
		void emit(DOCS_EVENT, payload)
	}

	#drop(label: string): void {
		if (!(label in this.elsewhere)) return
		const next = { ...this.elsewhere }
		delete next[label]
		this.elsewhere = next
	}
}

export const panels = new Panels()

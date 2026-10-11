import { tasks } from './tasks.svelte'

export type ViewId = 'content' | 'files' | 'search' | 'git' | 'debug'
export type PanelTab = 'problems' | 'output' | 'gamelog'

const VIEWS: ViewId[] = ['content', 'files', 'search', 'git', 'debug']
export const PANEL_TABS: PanelTab[] = ['problems', 'output', 'gamelog']

export function isPanelTab(kind: string): kind is PanelTab {
	return (PANEL_TABS as string[]).includes(kind)
}

function clamp(value: unknown, min: number, max: number, fallback: number): number {
	return typeof value === 'number' && Number.isFinite(value)
		? Math.min(max, Math.max(min, value))
		: fallback
}

class Layout {
	view = $state<ViewId>('content')
	sidebar = $state(true)
	sidebarWidth = $state(260)
	inspector = $state(true)
	inspectorWidth = $state(300)
	panel = $state(false)
	panelHeight = $state(190)
	panelTab = $state<PanelTab>('problems')
	#key: string | null = null

	restore(dir: string, remember = true): void {
		this.#key = remember ? `layout:${dir}` : null
		this.#apply(this.#key ? this.#read(this.#key) : { sidebar: false, inspector: false })
	}

	#read(key: string): Record<string, unknown> {
		try {
			return JSON.parse(localStorage.getItem(key) ?? '{}') as Record<string, unknown>
		} catch (error) {
			tasks.fail('Reading the saved layout', error)
			return {}
		}
	}

	#apply(saved: Record<string, unknown>): void {
		this.view = VIEWS.includes(saved.view as ViewId) ? (saved.view as ViewId) : 'content'
		this.sidebar = saved.sidebar !== false
		this.sidebarWidth = clamp(saved.sidebarWidth, 200, 480, 260)
		this.inspector = saved.inspector !== false
		this.inspectorWidth = clamp(saved.inspectorWidth, 240, 520, 300)
		this.panel = saved.panel === true
		this.panelHeight = clamp(saved.panelHeight, 120, 600, 190)
		this.panelTab = PANEL_TABS.includes(saved.panelTab as PanelTab)
			? (saved.panelTab as PanelTab)
			: 'problems'
	}

	persist(): void {
		if (!this.#key) return
		const data = {
			view: this.view,
			sidebar: this.sidebar,
			sidebarWidth: this.sidebarWidth,
			inspector: this.inspector,
			inspectorWidth: this.inspectorWidth,
			panel: this.panel,
			panelHeight: this.panelHeight,
			panelTab: this.panelTab,
		}
		try {
			localStorage.setItem(this.#key, JSON.stringify(data))
		} catch (error) {
			tasks.fail('Saving the layout', error)
		}
	}

	pickView(view: ViewId): void {
		if (this.view === view && this.sidebar) this.sidebar = false
		else {
			this.view = view
			this.sidebar = true
		}
		this.persist()
	}

	showView(view: ViewId): void {
		this.view = view
		this.sidebar = true
		this.persist()
	}

	toggleSidebar(): void {
		this.sidebar = !this.sidebar
		this.persist()
	}

	toggleInspector(): void {
		this.inspector = !this.inspector
		this.persist()
	}

	togglePanel(): void {
		this.panel = !this.panel
		this.persist()
	}

	openPanel(tab: PanelTab): void {
		this.panelTab = tab
		this.panel = true
		this.persist()
	}
}

export const layout = new Layout()

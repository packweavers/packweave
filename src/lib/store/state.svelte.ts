import type {
	GitStatus,
	Health,
	LocalContent,
	Lockfile,
	LockedMod,
	Manifest,
	ModMeta,
	ProviderInfo,
	SyncReport,
	Validation,
	VersionInfo,
} from '../../types'

export type ToastKind = 'success' | 'error' | 'warning' | 'info'

export interface Toast {
	id: number
	kind: ToastKind
	message: string
}

export interface OpenPack {
	dir: string
	manifest: Manifest
}

export interface RecentPack {
	dir: string
	name: string
	minecraft: string
	loader: string
	lastOpened: number
}

export type ThemePref = 'system' | 'light' | 'dark'
export type PackView = 'content' | 'files' | 'instance' | 'source'
export type GitProvider = 'github' | 'gitlab' | 'other'

export const s = $state({
	pack: null as OpenPack | null,
	lockfile: null as Lockfile | null,
	validations: [] as Validation[],
	health: null as Health | null,
	busy: false,
	toasts: [] as Toast[],
	toastSeq: 0,
	instanceDir: null as string | null,
	sync: null as SyncReport | null,
	scanning: false,
	syncPending: false,
	scanBusy: false,
	suppressAutoPush: false,
	git: null as GitStatus | null,
	updateInfo: null as { version: string; notes: string } | null,
	recents: [] as RecentPack[],
	theme: 'system' as ThemePref,
	view: 'content' as PackView,
	viewHistory: ['content'] as PackView[],
	viewIndex: 0,
	meta: {} as Record<string, ModMeta>,
	versions: {} as Record<string, VersionInfo[]>,
	latest: {} as Record<string, VersionInfo | null>,
	enriching: false,
	unpublished: [] as LocalContent[],
	autoPushOnSave: true,
	providers: [] as ProviderInfo[],
	settingsOpen: false,
	authPrompt: null as { provider: GitProvider } | null,
	deletePrompt: null as { ids: string[] } | null,
	prefs: {} as Record<string, unknown>,

	get hasPack(): boolean {
		return this.pack !== null
	},
	get hasLock(): boolean {
		return !!this.lockfile && this.lockfile.mods.length > 0
	},
	get presentIds(): Set<string> {
		return new Set((this.lockfile?.mods ?? []).map((m) => m.projectId))
	},
	get chosenMods(): LockedMod[] {
		return (this.lockfile?.mods ?? []).filter((m) => m.dependencyType === 'explicit')
	},
	get depMods(): LockedMod[] {
		return (this.lockfile?.mods ?? []).filter((m) => m.dependencyType === 'dependency')
	},
	get minecraft(): string {
		return this.pack?.manifest.minecraft ?? ''
	},
	get loader(): string {
		return this.pack?.manifest.loader ?? ''
	},
	get bound(): boolean {
		return this.instanceDir !== null
	},
	get hasClientOnly(): boolean {
		return (this.lockfile?.mods ?? []).some((m) => m.serverSide === 'unsupported')
	},
	get hasServerOnly(): boolean {
		return (this.lockfile?.mods ?? []).some((m) => m.clientSide === 'unsupported')
	},
	get syncChangeCount(): number {
		return this.sync ? this.sync.mods.length + this.sync.files.length + (this.sync.env ? 1 : 0) : 0
	},
	get canNavBack(): boolean {
		return this.viewIndex > 0
	},
	get canNavForward(): boolean {
		return this.viewIndex < this.viewHistory.length - 1
	},
	providerName(id: string): string {
		return this.providers.find((p) => p.id === id)?.displayName ?? id
	},
	get enabledProviders(): ProviderInfo[] {
		return this.providers.filter((p) => p.configured)
	},
})

export function notify(kind: ToastKind, message: string) {
	const id = ++s.toastSeq
	s.toasts.push({ id, kind, message })
	window.setTimeout(() => dismiss(id), 4600)
}

export function dismiss(id: number) {
	s.toasts = s.toasts.filter((t) => t.id !== id)
}

import {
	coversKey,
	opsFor,
	pausedWhileRunning,
	syncItems,
	type Direction,
	type SyncItem,
} from '@/lib/instance/items'
import { errorCode, errorText } from '@/lib/tauri/call'
import * as api from '@/lib/tauri/instance'
import type { DetectedInstance, Running, SyncIgnored, SyncReport } from '@/lib/tauri/types'

import { app } from './app.svelte'
import { bisect } from './bisect.svelte'
import { content } from './content.svelte'
import { panels } from './panels.svelte'
import { tasks } from './tasks.svelte'
import { toasts } from './toasts.svelte'
import { ws } from './workspace.svelte'

export type InstanceStatus =
	'none' | 'missing' | 'unknown' | 'running' | 'in-sync' | 'push' | 'pull' | 'both'

const RUNNING_POLL = 4000
const LOG_POLL = 1000

class InstanceState {
	binding = $state.raw<DetectedInstance | null>(null)
	report = $state.raw<SyncReport | null>(null)
	scanning = $state(false)
	missing = $state(false)
	scanError = $state<string | null>(null)
	running = $state.raw<Running | null>(null)
	applying = $state(false)
	log = $state('')
	logError = $state<string | null>(null)
	#logOffset = 0
	#scanTimer: ReturnType<typeof setTimeout> | null = null
	#lastScan = 0
	#runningTimer: ReturnType<typeof setInterval> | null = null
	#logTimer: ReturnType<typeof setInterval> | null = null

	get bound(): boolean {
		return this.binding !== null
	}

	get name(): string {
		return this.binding?.name ?? ''
	}

	get items(): SyncItem[] {
		return syncItems(this.report)
	}

	get active(): SyncItem[] {
		return this.items.filter((i) => !i.ignored)
	}

	get toPush(): SyncItem[] {
		return this.active.filter((i) => i.side === 'push')
	}

	get toPull(): SyncItem[] {
		return this.active.filter((i) => i.side === 'pull')
	}

	get both(): SyncItem[] {
		return this.active.filter((i) => i.side === 'both')
	}

	get ignored(): SyncIgnored[] {
		return this.report?.ignored ?? []
	}

	ignoredEntryFor(key: string): SyncIgnored | null {
		return this.ignored.find((entry) => coversKey(entry, key)) ?? null
	}

	async ignore(entries: SyncIgnored[]): Promise<void> {
		const dir = ws.dir
		if (!dir || entries.length === 0) return
		try {
			this.#useIgnored(await api.ignoreInSync(dir, entries))
			toasts.show(
				entries.length === 1
					? `Ignoring ${entries[0].label} for ${this.name}`
					: `Ignoring ${entries.length} for ${this.name}`,
				[{ label: 'Undo', run: () => void this.unignore(entries.map((e) => e.key)) }],
			)
		} catch (error) {
			toasts.error(error)
		}
	}

	async unignore(keys: string[]): Promise<void> {
		const dir = ws.dir
		if (!dir || keys.length === 0) return
		try {
			this.#useIgnored(await api.unignoreInSync(dir, keys))
		} catch (error) {
			toasts.error(error)
		}
	}

	#useIgnored(list: SyncIgnored[]): void {
		const report = this.report
		if (!report) return
		const ignored = (key: string) => list.some((entry) => coversKey(entry, key))
		const mods = report.mods.map((m) => ({ ...m, ignored: ignored(m.key) }))
		const files = report.files.map((f) => ({ ...f, ignored: ignored(f.key) }))
		const env = report.env ? { ...report.env, ignored: ignored(report.env.key) } : null
		this.report = {
			...report,
			mods,
			files,
			env,
			ignored: list,
			inSync: [...mods, ...files, ...(env ? [env] : [])].every((change) => change.ignored),
		}
	}

	get status(): InstanceStatus {
		if (!this.binding) return 'none'
		if (this.running) return 'running'
		if (this.missing) return 'missing'
		if (!this.report) return 'unknown'
		const push = this.toPush.length > 0
		const pull = this.toPull.length > 0 || this.both.length > 0
		if (push && pull) return 'both'
		if (push) return 'push'
		if (pull) return 'pull'
		return 'in-sync'
	}

	reset(): void {
		this.binding = null
		this.report = null
		this.missing = false
		this.scanError = null
		this.running = null
		this.log = ''
		this.#logOffset = 0
		this.#stopTimers()
	}

	async load(): Promise<void> {
		const dir = ws.dir
		if (!dir) return
		this.binding = await api.binding(dir).catch(tasks.or('Reading the linked instance', null))
		bisect.restore()
		if (!this.binding) return
		this.#startRunningPoll()
		await this.scan()
	}

	async scan(): Promise<void> {
		const dir = ws.dir
		if (!dir || !this.binding || this.scanning) return
		this.scanning = true
		this.#lastScan = Date.now()
		try {
			const report = await tasks.run(`Scanning ${this.binding.name}`, () => api.syncStatus(dir))
			if (ws.dir !== dir) return
			this.report = report
			this.missing = false
			this.scanError = null
		} catch (error) {
			this.missing = errorCode(error) === 'instance-missing'
			this.scanError = errorText(error)
		} finally {
			this.scanning = false
		}
	}

	scanIfStale(maxAge: number): void {
		if (Date.now() - this.#lastScan > maxAge) this.scheduleScan()
	}

	scheduleScan(): void {
		if (!this.binding) return
		if (this.#scanTimer) clearTimeout(this.#scanTimer)
		this.#scanTimer = setTimeout(() => void this.scan(), 600)
	}

	async unlink(): Promise<void> {
		const dir = ws.dir
		if (!dir) return
		await api.unbindInstance(dir)
		this.reset()
	}

	async apply(
		items: SyncItem[],
		directions: Direction[],
		label: string,
		done: (count: number) => string,
	): Promise<boolean> {
		const dir = ws.dir
		if (!dir || items.length === 0) return false
		if (bisect.active) {
			toasts.info('Stop Find a broken mod first. It has mods turned off in this instance.')
			return false
		}
		if (this.running) {
			const allowed = items.map((item, i) => !pausedWhileRunning(item, directions[i]))
			if (!allowed.includes(true)) {
				toasts.info(`Close ${this.name} to sync mods`)
				return false
			}
			items = items.filter((_, i) => allowed[i])
			directions = directions.filter((_, i) => allowed[i])
		}
		this.applying = true
		try {
			const ops = opsFor(items, directions)
			const resolved = await tasks.run(label, () => api.applySync(dir, ops, label))
			if (ws.dir !== dir) return false
			content.apply(resolved)
			void content.refreshHistory()
			ws.changed()
			toasts.show(done(items.length), [
				{ label: 'Undo', command: 'edit.undo', run: () => void content.undo() },
			])
			return true
		} catch (error) {
			toasts.error(error)
			return false
		} finally {
			this.applying = false
			await this.scan()
		}
	}

	push(items: SyncItem[] = this.toPush): Promise<boolean> {
		return this.apply(
			items,
			items.map(() => 'push'),
			`Pushing to ${this.name}`,
			(n) => `Pushed ${n} to ${this.name}`,
		)
	}

	pull(items: SyncItem[] = this.toPull): Promise<boolean> {
		return this.apply(
			items,
			items.map(() => 'pull'),
			`Pulling from ${this.name}`,
			(n) => `Pulled ${n} from ${this.name}`,
		)
	}

	revert(items: SyncItem[]): Promise<boolean> {
		const reversible = items.filter((i) => i.side !== 'both')
		return this.apply(
			reversible,
			reversible.map((i) => (i.side === 'push' ? 'pull' : 'push')),
			'Reverting',
			(n) => `Reverted ${n}`,
		)
	}

	async pollRunning(): Promise<void> {
		const dir = ws.dir
		if (!dir || !this.binding) return
		const was = this.running
		this.running = await api
			.running(dir)
			.catch(tasks.or('Checking whether the game is running', null))
		if (was && !this.running) this.scheduleScan()
		if (!was && this.running && app.prefs.gameLogOnLaunch) panels.open('gamelog')
	}

	followLog(on: boolean): void {
		if (this.#logTimer) clearInterval(this.#logTimer)
		this.#logTimer = null
		if (!on) return
		void this.readLog()
		this.#logTimer = setInterval(() => void this.readLog(), LOG_POLL)
	}

	async readLog(): Promise<void> {
		const dir = ws.dir
		if (!dir || !this.binding) return
		try {
			const chunk = await api.readGameLog(dir, this.#logOffset)
			if (chunk.reset) this.log = ''
			if (chunk.text) this.log = (this.log + chunk.text).slice(-400_000)
			this.#logOffset = chunk.offset
			this.logError = null
		} catch (error) {
			this.logError = errorText(error)
		}
	}

	#startRunningPoll(): void {
		if (this.#runningTimer) clearInterval(this.#runningTimer)
		void this.pollRunning()
		this.#runningTimer = setInterval(() => void this.pollRunning(), RUNNING_POLL)
	}

	#stopTimers(): void {
		if (this.#scanTimer) clearTimeout(this.#scanTimer)
		if (this.#runningTimer) clearInterval(this.#runningTimer)
		if (this.#logTimer) clearInterval(this.#logTimer)
		this.#scanTimer = null
		this.#runningTimer = null
		this.#logTimer = null
	}
}

export const instance = new InstanceState()

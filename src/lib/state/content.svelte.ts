import { contentChange, type ContentChange } from '@/lib/content/changes'
import type { RowInputs } from '@/lib/content/rows'
import { hasSource } from '@/lib/content/sources'
import { enrich } from '@/lib/tauri/catalog'
import {
	addAltSources,
	addContent,
	checkUpdates,
	listUnpublished,
	recheckDroppedFiles,
	redo,
	removeAltSource,
	removeContent,
	resolvePack,
	setBundled,
	setDisabled,
	setExplicit,
	setPinned,
	setPreferredSource,
	setSourceVersion,
	setVersions,
	undo,
	undoState,
} from '@/lib/tauri/content'
import { packDiffWorking } from '@/lib/tauri/git'
import type {
	AddItem,
	AvailableUpdate,
	Health,
	ItemChange,
	LocalContent,
	LockedMod,
	Lockfile,
	ModMeta,
	PackDiff,
	PackResolved,
	ProjectType,
	ProviderId,
	Suggested,
	UndoState,
	Validation,
	VersionPin,
} from '@/lib/tauri/types'
import { listNames, plural, providerName, typeName } from '@/lib/util/format'

import { app } from './app.svelte'
import { tasks } from './tasks.svelte'
import { toasts } from './toasts.svelte'
import { ws } from './workspace.svelte'

function problemKey(dir: string, v: Validation): string {
	return [dir, v.key].join('\u0000')
}

export function suggestedItem(suggestion: Suggested): AddItem {
	return {
		provider: suggestion.provider,
		projectId: suggestion.projectId,
		slug: suggestion.slug,
		name: suggestion.name,
		projectType: 'mod',
	}
}

class ContentState {
	lockfile = $state.raw<Lockfile | null>(null)
	validations = $state.raw<Validation[]>([])
	health = $state.raw<Health | null>(null)
	meta = $state<Record<string, ModMeta>>({})
	updates = $state<Record<string, AvailableUpdate>>({})
	changes = $state.raw<PackDiff | null>(null)
	unpublished = $state.raw<LocalContent[]>([])
	suggested = $state.raw<Suggested[]>([])
	resolving = $state(false)
	checking = $state(false)
	history = $state<UndoState>({ undo: null, redo: null })
	#rechecking = false
	#lastRecheck = 0

	get mods(): LockedMod[] {
		return this.lockfile?.mods ?? []
	}

	ofType(type: ProjectType): LockedMod[] {
		return this.mods.filter((m) => m.projectType === type)
	}

	find(key: string): LockedMod | undefined {
		return (
			this.mods.find((m) => m.key === key) ??
			this.mods.find((m) => m.projectId === key) ??
			this.mods.find((m) => Object.values(m.sources).some((s) => s?.id === key))
		)
	}

	holder(provider: ProviderId, id: string): LockedMod | undefined {
		return this.mods.find((m) => hasSource(m, provider, id))
	}

	changeOf(key: string): ItemChange | undefined {
		return this.changes?.items.find((c) => c.path === key)
	}

	changeWordOf(key: string): ContentChange | null {
		const change = this.changeOf(key)
		return change ? contentChange(change.kind) : null
	}

	requiredBy(key: string): LockedMod[] {
		const mod = this.find(key)
		if (!mod) return []
		return mod.dependents.map((id) => this.find(id)).filter((m): m is LockedMod => !!m)
	}

	bringsIn(key: string): LockedMod[] {
		const mod = this.find(key)
		return mod ? this.mods.filter((m) => m.dependents.includes(mod.projectId)) : []
	}

	optionalIn(key: string): LockedMod[] {
		const mod = this.find(key)
		return mod ? this.mods.filter((m) => m.optionalDependents.includes(mod.projectId)) : []
	}

	optionalMissing(key: string): Suggested[] {
		const mod = this.find(key)
		return mod ? this.suggested.filter((s) => s.wantedBy.includes(mod.projectId)) : []
	}

	addSuggested(suggestion: Suggested): Promise<boolean> {
		return this.add([suggestedItem(suggestion)])
	}

	get rowInputs(): RowInputs {
		const present = new Set(this.mods.map((m) => m.key))
		const problemKeys = new Set(
			this.problems
				.map((p) => (p.projectId ? this.find(p.projectId)?.key : undefined))
				.filter((key): key is string => !!key),
		)
		return {
			mods: this.mods,
			removed: (this.changes?.items ?? []).filter(
				(c) => c.kind === 'removed' && !present.has(c.path),
			),
			local: this.unpublished,
			meta: this.meta,
			updates: this.updates,
			changeOf: (key) => this.changeWordOf(key),
			problemKeys,
			nameOf: (id) => this.find(id)?.name,
		}
	}

	get updateCount(): number {
		return Object.keys(this.updates).length
	}

	get problems(): Validation[] {
		const dir = ws.dir ?? ''
		const hidden = new Set(app.prefs.hiddenProblems)
		return this.validations.filter((v) => v.severity !== 'ok' && !hidden.has(problemKey(dir, v)))
	}

	get hiddenCount(): number {
		const dir = ws.dir ?? ''
		const hidden = new Set(app.prefs.hiddenProblems)
		return this.validations.filter((v) => v.severity !== 'ok' && hidden.has(problemKey(dir, v)))
			.length
	}

	hideProblem(v: Validation): void {
		if (!ws.dir) return
		app.prefs.hiddenProblems = [...app.prefs.hiddenProblems, problemKey(ws.dir, v)]
		app.save()
	}

	showHiddenProblems(): void {
		if (!ws.dir) return
		const prefix = `${ws.dir}\u0000`
		app.prefs.hiddenProblems = app.prefs.hiddenProblems.filter((k) => !k.startsWith(prefix))
		app.save()
	}

	reset(): void {
		this.lockfile = null
		this.validations = []
		this.health = null
		this.meta = {}
		this.updates = {}
		this.changes = null
		this.unpublished = []
		this.suggested = []
		this.history = { undo: null, redo: null }
	}

	applyLockfile(lockfile: Lockfile | null): void {
		this.lockfile = lockfile
		void this.enrichMissing()
	}

	apply(resolved: PackResolved): void {
		this.lockfile = resolved.lockfile
		this.validations = resolved.validations
		this.health = resolved.health
		this.suggested = resolved.suggested
		if (ws.manifest) ws.manifest = resolved.manifest
		ws.ensureLoaderVersion()
		for (const key of Object.keys(this.updates)) {
			const mod = this.find(key)
			const update = this.updates[key]
			if (
				!mod ||
				mod.pinned ||
				mod.projectId !== update.projectId ||
				mod.sources[mod.preferred]?.versionId === update.versionId
			) {
				delete this.updates[key]
			}
		}
		void this.enrichMissing()
	}

	async warmUp(): Promise<void> {
		await this.resolve(false, true)
		void this.recheckDropped(true)
		void this.refreshChanges()
		void this.refreshUnpublished()
		void this.refreshHistory()
		void this.checkUpdates()
	}

	async resolve(force = false, quiet = false): Promise<void> {
		const dir = ws.dir
		if (!dir) return
		this.resolving = true
		try {
			const resolved = await tasks.run(force ? 'Resolving again' : 'Resolving', () =>
				resolvePack(dir, force),
			)
			if (ws.dir === dir) this.apply(resolved)
		} catch (error) {
			if (!quiet) toasts.error(error)
		} finally {
			this.resolving = false
		}
	}

	async refreshChanges(): Promise<void> {
		const dir = ws.dir
		if (!dir) return
		try {
			const changes = await packDiffWorking(dir)
			if (ws.dir === dir) this.changes = changes
		} catch (error) {
			tasks.fail('Checking what changed', error)
			if (ws.dir === dir) this.changes = null
		}
	}

	async refreshUnpublished(): Promise<void> {
		const dir = ws.dir
		if (!dir) return
		this.unpublished = await listUnpublished(dir).catch(tasks.or('Listing loose content files', []))
	}

	async refreshHistory(): Promise<void> {
		if (!ws.dir) return
		this.history = await undoState(ws.dir).catch(
			tasks.or('Reading undo history', { undo: null, redo: null }),
		)
	}

	async enrichMissing(): Promise<void> {
		const missing = this.mods
			.filter((m) => !this.meta[m.projectId])
			.filter((m) => m.preferred === 'modrinth' || m.preferred === 'curseforge')
			.map((m) => ({ id: m.projectId, provider: m.preferred }))
		if (missing.length === 0) return
		try {
			for (const meta of await enrich(missing)) this.meta[meta.projectId] = meta
		} catch (error) {
			tasks.fail('Loading icons and authors', error)
		}
	}

	async checkUpdates(loud = false): Promise<void> {
		const dir = ws.dir
		if (!dir || this.checking) return
		this.checking = true
		try {
			const found = await tasks.run('Checking for updates', () => checkUpdates(dir))
			if (ws.dir !== dir) return
			this.updates = Object.fromEntries(found.map((u) => [u.key, u]))
		} catch (error) {
			if (loud) toasts.error(error)
		} finally {
			this.checking = false
		}
	}

	async #mutate(
		label: string,
		message: string | (() => string),
		work: (dir: string) => Promise<PackResolved>,
	): Promise<boolean> {
		const dir = ws.dir
		if (!dir) return false
		try {
			const resolved = await tasks.run(label, () => work(dir))
			if (ws.dir !== dir) return false
			this.apply(resolved)
			ws.changed()
			void this.refreshHistory()
			const text = typeof message === 'string' ? message : message()
			toasts.show(text, [{ label: 'Undo', command: 'edit.undo', run: () => void this.undo() }])
			return true
		} catch (error) {
			toasts.error(error)
			return false
		}
	}

	#names(keys: string[]): string {
		return listNames(keys.map((k) => this.find(k)?.name ?? k))
	}

	add(items: AddItem[]): Promise<boolean> {
		const names = listNames(items.map((i) => i.name || i.slug || i.projectId))
		const added = () => {
			const moved = items
				.map((i) => this.find(i.projectId))
				.filter((m, n): m is LockedMod => !!m && m.projectType !== items[n].projectType)
			if (moved.length === 0) return `Added ${names}`
			if (items.length === 1) {
				const [m] = moved
				return `Added ${m.name} to ${typeName(m.projectType, true)}: its file is a ${typeName(m.projectType).toLowerCase()}`
			}
			const where = moved.map((m) => `${m.name} to ${typeName(m.projectType, true)}`)
			return `Added ${names}. Moved to match their files: ${where.join(', ')}`
		}
		return this.#mutate('Adding content', added, (dir) => addContent(dir, items))
	}

	remove(keys: string[]): Promise<boolean> {
		const names = this.#names(keys)
		return this.#mutate('Removing', `Removed ${names}`, (dir) => removeContent(dir, keys))
	}

	setDisabled(keys: string[], disabled: boolean): Promise<boolean> {
		const names = this.#names(keys)
		const [label, done] = disabled ? ['Disabling', 'Disabled'] : ['Enabling', 'Enabled']
		return this.#mutate(label, `${done} ${names}`, (dir) => setDisabled(dir, keys, disabled))
	}

	setPinned(keys: string[], pinned: boolean): Promise<boolean> {
		const names = this.#names(keys)
		const verb = pinned ? 'Pinned' : 'Unpinned'
		return this.#mutate(verb, `${verb} ${names}`, (dir) => setPinned(dir, keys, pinned))
	}

	setSourceVersion(key: string, provider: ProviderId, version: string | null): Promise<boolean> {
		const name = this.#names([key])
		return this.#mutate(
			'Updating',
			version ? `Pinned ${name} on ${providerName(provider)}` : `${name} matches again`,
			(dir) => setSourceVersion(dir, key, provider, version),
		)
	}

	setBundled(keys: string[], bundled: boolean): Promise<boolean> {
		const names = this.#names(keys)
		const done = bundled ? `Bundling ${names} in exports` : `Leaving ${names} out of exports`
		return this.#mutate('Updating', done, (dir) => setBundled(dir, keys, bundled))
	}

	setExplicit(key: string, explicit: boolean): Promise<boolean> {
		const name = this.#names([key])
		const message = explicit ? `${name} stays in the pack` : `${name} is now a dependency`
		return this.#mutate('Updating', message, (dir) => setExplicit(dir, key, explicit))
	}

	setVersions(pins: VersionPin[]): Promise<boolean> {
		if (pins.length === 0) return Promise.resolve(false)
		const names = this.#names(pins.map((p) => p.key))
		return this.#mutate('Updating', `Updated ${names}`, (dir) => setVersions(dir, pins))
	}

	pinsFor(keys: string[]): VersionPin[] {
		return keys
			.map((key) => this.updates[key])
			.filter((u): u is AvailableUpdate => !!u)
			.map((u) => ({ key: u.key, version: u.versionId }))
	}

	async preferSource(keys: string[], provider: ProviderId): Promise<boolean> {
		const done = await this.#mutate(
			'Switching source',
			`${this.#names(keys)} now ${keys.length === 1 ? 'uses' : 'use'} ${providerName(provider)}`,
			(dir) => setPreferredSource(dir, keys, provider),
		)
		if (done) void this.checkUpdates()
		return done
	}

	addSources(sources: { key: string; provider: ProviderId; id: string }[]): Promise<boolean> {
		return this.#mutate(
			'Adding sources',
			`Added sources to ${plural(sources.length, 'mod', 'mods')}`,
			(dir) => addAltSources(dir, sources),
		)
	}

	removeSource(key: string, provider: ProviderId): Promise<boolean> {
		return this.#mutate(
			'Removing source',
			`Removed ${providerName(provider)} from ${this.#names([key])}`,
			(dir) => removeAltSource(dir, key, provider),
		)
	}

	async recheckDropped(force = false): Promise<void> {
		const dir = ws.dir
		if (!dir || ws.detached || this.#rechecking || tasks.running.length > 0) return
		if (!force && Date.now() - this.#lastRecheck < 60_000) return
		this.#rechecking = true
		this.#lastRecheck = Date.now()
		try {
			const result = await recheckDroppedFiles(dir)
			if (ws.dir !== dir || !result.resolved || result.found.length === 0) return
			this.apply(result.resolved)
			ws.changed()
			void this.refreshHistory()
			void this.refreshUnpublished()
			const platforms = [...new Set(result.found.map((f) => providerName(f.provider)))]
			toasts.show(
				`Found ${listNames(result.found.map((f) => f.name))} on ${platforms.join(' and ')}`,
				[{ label: 'Undo', command: 'edit.undo', run: () => void this.undo() }],
			)
		} catch (error) {
			tasks.fail('Looking up dropped files', error)
		} finally {
			this.#rechecking = false
		}
	}

	async undo(): Promise<void> {
		await this.#step(true)
	}

	async redo(): Promise<void> {
		await this.#step(false)
	}

	async #step(back: boolean): Promise<void> {
		const dir = ws.dir
		if (!dir) return
		try {
			const result = await (back ? undo(dir) : redo(dir))
			if (!result) {
				toasts.info(back ? 'Nothing to undo' : 'Nothing to redo')
				return
			}
			if (result.resolved) this.apply(result.resolved)
			ws.changed()
			void this.refreshHistory()
			toasts.show(`${back ? 'Undid' : 'Redid'} "${result.label}"`, [
				back
					? { label: 'Redo', command: 'edit.redo', run: () => void this.redo() }
					: { label: 'Undo', command: 'edit.undo', run: () => void this.undo() },
			])
		} catch (error) {
			toasts.error(error)
		}
	}
}

export const content = new ContentState()

import {
	answer as answerStep,
	back as backStep,
	begin,
	enabledFor,
	type BisectMod,
	type BisectSession,
} from '@/lib/debug/bisect'
import { instanceJars, setInstanceJars } from '@/lib/tauri/instance'
import type { LockedMod } from '@/lib/tauri/types'

import { content } from './content.svelte'
import { instance } from './instance.svelte'
import { tasks } from './tasks.svelte'
import { toasts } from './toasts.svelte'
import { ws } from './workspace.svelte'

function keyFor(gameDir: string): string {
	return `bisect:${gameDir}`
}

interface SavedMod {
	file: string
	name: string
	needs: string[]
	key?: string | null
	projectId?: string | null
}

function upgrade(session: BisectSession): BisectSession {
	return {
		...session,
		mods: (session.mods as SavedMod[]).map((mod) => ({
			file: mod.file,
			name: mod.name,
			needs: mod.needs,
			key: mod.key ?? (mod.projectId ? (content.find(mod.projectId)?.key ?? null) : null),
		})),
	}
}

class Bisect {
	session = $state.raw<BisectSession | null>(null)
	busy = $state(false)

	get active(): boolean {
		return this.session !== null
	}

	restore(): void {
		const gameDir = instance.binding?.gameDir
		if (!gameDir) {
			this.session = null
			return
		}
		try {
			const saved = localStorage.getItem(keyFor(gameDir))
			this.session = saved ? upgrade(JSON.parse(saved) as BisectSession) : null
		} catch (error) {
			tasks.fail('Reading the saved Find a broken mod session', error)
			this.session = null
		}
	}

	async collect(): Promise<{ mods: BisectMod[]; original: string[] } | null> {
		const dir = ws.dir
		if (!dir) return null
		const jars = await instanceJars(dir)
		const original = jars.filter((j) => j.enabled).map((j) => j.file)
		const owner = new Map<string, LockedMod>()
		for (const mod of content.mods) {
			for (const source of Object.values(mod.sources)) {
				if (source?.filename) owner.set(source.filename, mod)
			}
		}
		const fileOf = new Map<string, string>()
		for (const file of original) {
			const mod = owner.get(file)
			if (mod) fileOf.set(mod.key, file)
		}
		const mods = original.map((file) => {
			const mod = owner.get(file)
			const needs = mod
				? content
						.bringsIn(mod.key)
						.map((dep) => fileOf.get(dep.key))
						.filter((f): f is string => !!f)
				: []
			return {
				file,
				name: mod?.name ?? file.replace(/\.jar$/i, ''),
				key: mod?.key ?? null,
				needs,
			}
		})
		mods.sort((a, b) => a.name.localeCompare(b.name))
		return { mods, original }
	}

	async start(
		mods: BisectMod[],
		original: string[],
		suspects: string[],
		keep: string[],
	): Promise<void> {
		const gameDir = instance.binding?.gameDir
		if (!gameDir) return
		const ordered = mods.map((m) => m.file).filter((f) => suspects.includes(f))
		await this.#use(begin(gameDir, mods, original, ordered, keep))
	}

	async answer(happened: boolean): Promise<void> {
		if (this.session) await this.#use(answerStep(this.session, happened))
	}

	async back(): Promise<void> {
		if (this.session) await this.#use(backStep(this.session))
	}

	async check(): Promise<void> {
		if (this.session) await this.#use({ ...this.session, checking: true, checked: null })
	}

	async stop(): Promise<void> {
		const dir = ws.dir
		const session = this.session
		if (!dir || !session) return
		this.busy = true
		try {
			await setInstanceJars(dir, session.original, [])
			this.#save(null, session.instanceDir)
			toasts.show(`Restored ${instance.name}`)
			void instance.scan()
		} catch (error) {
			toasts.error(error)
		} finally {
			this.busy = false
		}
	}

	async #use(next: BisectSession): Promise<void> {
		const dir = ws.dir
		if (!dir) return
		this.busy = true
		try {
			const on = enabledFor(next)
			const enable = next.original.filter((f) => on.has(f))
			const disable = next.original.filter((f) => !on.has(f))
			await setInstanceJars(dir, enable, disable)
			this.#save(next, next.instanceDir)
		} catch (error) {
			toasts.error(error)
		} finally {
			this.busy = false
		}
	}

	#save(next: BisectSession | null, gameDir: string): void {
		this.session = next
		try {
			if (next) {
				localStorage.setItem(keyFor(gameDir), JSON.stringify(next))
			} else {
				localStorage.removeItem(keyFor(gameDir))
			}
		} catch (error) {
			tasks.fail('Saving Find a broken mod progress', error)
		}
	}
}

export const bisect = new Bisect()

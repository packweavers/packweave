import {
	fileRule,
	fileUnlocked,
	parseRule,
	planRule,
	settingUnlocked,
	type Rule,
} from '@/lib/files/rules'
import { preservePatterns, preserveReset, preserveUpdate } from '@/lib/tauri/pack'

import { tasks } from './tasks.svelte'
import { toasts } from './toasts.svelte'
import { ws } from './workspace.svelte'

export const PRESERVE_FILE = '.packweaveunlock'

class Preserve {
	patterns = $state<string[]>([])
	rules = $derived(this.patterns.map(parseRule).filter((r): r is Rule => r !== null))

	fileRule(rel: string): Rule | null {
		return fileRule(this.rules, rel)
	}

	fileUnlocked(rel: string): boolean {
		return fileUnlocked(this.rules, rel)
	}

	settingUnlocked(rel: string, key: string): boolean {
		return settingUnlocked(this.rules, rel, key)
	}

	async load(): Promise<void> {
		const dir = ws.dir
		if (!dir) return
		this.patterns = await preservePatterns(dir).catch(tasks.or('Reading the player edit rules', []))
	}

	setFile(path: string, unlock: boolean): Promise<void> {
		const plan = planRule(this.rules, path, null, unlock)
		return this.#update(plan.remove, plan.add)
	}

	setSetting(path: string, key: string, unlock: boolean): Promise<void> {
		const plan = planRule(this.rules, path, key, unlock)
		return this.#update(plan.remove, plan.add)
	}

	async add(line: string): Promise<boolean> {
		const rule = parseRule(line)
		if (!rule) {
			toasts.info(`Can't read ${line.trim()}`)
			return false
		}
		await this.#update([], [rule.line])
		return true
	}

	remove(lines: string[]): Promise<void> {
		return this.#update(lines, [])
	}

	async reset(): Promise<void> {
		const dir = ws.dir
		if (!dir) return
		try {
			this.patterns = await preserveReset(dir)
			ws.changed()
			toasts.show('Player edits reset to defaults')
		} catch (error) {
			toasts.error(error)
		}
	}

	async #update(remove: string[], add: string[]): Promise<void> {
		const dir = ws.dir
		if (!dir || (remove.length === 0 && add.length === 0)) return
		try {
			this.patterns = await preserveUpdate(dir, remove, add)
			ws.changed()
		} catch (error) {
			toasts.error(error)
		}
	}
}

export const preserve = new Preserve()

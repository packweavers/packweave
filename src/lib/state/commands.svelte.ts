import type { Component } from 'svelte'

import { app } from './app.svelte'
import { comboOf, effectiveKeys, formatCombo, normalizeCombo } from './keymap'

export interface Command {
	id: string
	title: string
	group: string
	icon?: Component
	when?: () => boolean
	whileTyping?: boolean
	run: () => unknown
}

const registry = new Map<string, Command>()

export function registerCommands(list: Command[]): void {
	for (const command of list) registry.set(command.id, command)
}

export function allCommands(): Command[] {
	return [...registry.values()]
}

export function isAvailable(command: Command): boolean {
	return command.when ? command.when() : true
}

function currentKeys(): Record<string, string> {
	return effectiveKeys(app.prefs.keymap.preset, app.prefs.keymap.custom)
}

export function comboFor(id: string): string | null {
	return currentKeys()[id] ?? null
}

export function keysFor(id: string): string | null {
	const combo = comboFor(id)
	return combo ? formatCombo(combo) : null
}

export function runCommand(id: string): boolean {
	const command = registry.get(id)
	if (!command || !isAvailable(command)) return false
	void command.run()
	return true
}

function isTyping(target: EventTarget | null): boolean {
	if (!(target instanceof HTMLElement)) return false
	return target.isContentEditable || ['INPUT', 'TEXTAREA', 'SELECT'].includes(target.tagName)
}

export function handleKey(event: KeyboardEvent): boolean {
	const combo = comboOf(event)
	if (!combo) return false
	const typing = isTyping(event.target)
	if (typing && !combo.includes('+') && !/^F\d+$/.test(combo)) return false
	for (const [id, keys] of Object.entries(currentKeys())) {
		if (normalizeCombo(keys) !== combo) continue
		const command = registry.get(id)
		if (!command || !isAvailable(command)) continue
		if (typing && command.whileTyping === false) continue
		event.preventDefault()
		void command.run()
		return true
	}
	return false
}

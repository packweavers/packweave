import { isMac } from '@/lib/util/platform'

import type { KeymapPreset } from './prefs'

const PACKWEAVE: Record<string, string> = {
	'palette.open': 'Mod+K',
	'palette.goto': 'Mod+P',
	'palette.commands': 'Mod+Shift+P',
	'pack.open': 'Mod+O',
	'app.settings': 'Mod+Comma',
	'pack.settings': 'Mod+Semicolon',
	'edit.undo': 'Mod+Z',
	'edit.redo': 'Mod+Shift+Z',
	'branch.openWindow': 'Mod+Shift+O',
	'branch.new': 'Mod+Shift+N',
	'tab.close': 'Mod+W',
	'tab.reopen': 'Mod+Shift+T',
	'tab.next': 'Ctrl+Tab',
	'tab.previous': 'Ctrl+Shift+Tab',
	'editor.split': 'Mod+Backslash',
	'window.close': 'Mod+Shift+W',
	'view.content': 'Mod+Shift+C',
	'view.files': 'Mod+Shift+E',
	'view.search': 'Mod+Shift+F',
	'view.git': 'Mod+Shift+G',
	'panel.problems': 'Mod+Shift+M',
	'layout.sidebar': 'Mod+B',
	'layout.panel': 'Mod+J',
	'layout.inspector': 'Mod+Alt+B',
	'content.add': 'Mod+N',
	'content.update': 'Mod+U',
	'content.updateAll': 'Mod+Shift+U',
	'content.remove': 'Mod+Backspace',
	'file.save': 'Mod+S',
	'git.commit': 'Mod+Enter',
	'git.push': 'Mod+Alt+Up',
	'git.pull': 'Mod+Alt+Down',
	'instance.push': 'Mod+Shift+R',
	'instance.pull': 'Mod+Alt+R',
}

for (let n = 1; n <= 9; n++) PACKWEAVE[`tab.goto${n}`] = `Mod+${n}`

const CHANGES: Record<KeymapPreset, Record<string, string | null>> = {
	packweave: {},
	vscode: {
		'git.push': null,
		'git.pull': null,
	},
	jetbrains: {
		'palette.commands': 'Mod+Shift+A',
		'git.commit': 'Mod+K',
		'palette.goto': 'Mod+Shift+O',
		'branch.openWindow': null,
		'git.push': 'Mod+Shift+K',
		'git.pull': 'Mod+T',
		'layout.sidebar': 'Mod+1',
		'tab.goto1': null,
	},
}

export function presetKeys(preset: KeymapPreset): Record<string, string> {
	const keys = { ...PACKWEAVE }
	for (const [id, combo] of Object.entries(CHANGES[preset])) {
		if (combo === null) {
			delete keys[id]
		} else {
			keys[id] = combo
		}
	}
	return keys
}

export function effectiveKeys(
	preset: KeymapPreset,
	custom: Record<string, string>,
): Record<string, string> {
	const keys = presetKeys(preset)
	for (const [id, combo] of Object.entries(custom)) {
		if (combo === '') {
			delete keys[id]
		} else {
			keys[id] = combo
		}
	}
	return keys
}

const CODES: Record<string, string> = {
	Comma: 'Comma',
	Period: 'Period',
	Semicolon: 'Semicolon',
	Slash: 'Slash',
	Backslash: 'Backslash',
	BracketLeft: 'BracketLeft',
	BracketRight: 'BracketRight',
	Minus: 'Minus',
	Equal: 'Equal',
	Quote: 'Quote',
	Backquote: 'Backquote',
	Enter: 'Enter',
	NumpadEnter: 'Enter',
	Backspace: 'Backspace',
	Delete: 'Delete',
	Space: 'Space',
	Tab: 'Tab',
	Escape: 'Escape',
	ArrowUp: 'Up',
	ArrowDown: 'Down',
	ArrowLeft: 'Left',
	ArrowRight: 'Right',
	Home: 'Home',
	End: 'End',
	PageUp: 'PageUp',
	PageDown: 'PageDown',
}

function keyName(event: KeyboardEvent): string | null {
	const code = event.code
	if (/^Key[A-Z]$/.test(code)) return code.slice(3)
	if (/^Digit\d$/.test(code)) return code.slice(5)
	if (/^Numpad\d$/.test(code)) return code.slice(6)
	if (/^F\d{1,2}$/.test(code)) return code
	return CODES[code] ?? null
}

export function comboOf(event: KeyboardEvent): string | null {
	const key = keyName(event)
	if (!key) return null
	const parts: string[] = []
	if (isMac ? event.metaKey : event.ctrlKey) parts.push('Mod')
	if (isMac && event.ctrlKey) parts.push('Ctrl')
	if (event.altKey) parts.push('Alt')
	if (event.shiftKey) parts.push('Shift')
	parts.push(key)
	return parts.join('+')
}

export function normalizeCombo(combo: string): string {
	const parts = combo.split('+')
	const key = parts.pop() ?? ''
	const mods = new Set(parts.map((p) => (!isMac && p === 'Ctrl' ? 'Mod' : p)))
	const order = ['Mod', 'Ctrl', 'Alt', 'Shift'].filter((m) => mods.has(m))
	return [...order, key].join('+')
}

const MAC_SYMBOLS: Record<string, string> = {
	Mod: '⌘',
	Ctrl: '⌃',
	Alt: '⌥',
	Shift: '⇧',
}

const PUNCTUATION: Record<string, string> = {
	Comma: ',',
	Period: '.',
	Semicolon: ';',
	Slash: '/',
	Backslash: '\\',
	BracketLeft: '[',
	BracketRight: ']',
	Minus: '-',
	Equal: '=',
	Quote: "'",
	Backquote: '`',
	Up: '↑',
	Down: '↓',
	Left: '←',
	Right: '→',
}

const MAC_KEYS: Record<string, string> = {
	Enter: '↵',
	Backspace: '⌫',
	Delete: '⌦',
	Tab: '⇥',
	Escape: 'esc',
	Space: 'Space',
}

const OTHER_KEYS: Record<string, string> = {
	Escape: 'Esc',
	Delete: 'Del',
}

export function formatCombo(combo: string): string {
	const parts = normalizeCombo(combo).split('+')
	const key = parts.pop() ?? ''
	if (isMac) {
		const label = PUNCTUATION[key] ?? MAC_KEYS[key] ?? key
		return parts.map((p) => MAC_SYMBOLS[p] ?? p).join('') + label
	}
	const label = PUNCTUATION[key] ?? OTHER_KEYS[key] ?? key
	return [...parts.map((p) => (p === 'Mod' ? 'Ctrl' : p)), label].join('+')
}

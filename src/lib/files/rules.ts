import { globMatches } from '@/lib/util/path'

export interface Rule {
	line: string
	unlock: boolean
	path: string
	key: string | null
}

export type RuleKind = 'folder' | 'file' | 'glob' | 'setting'

export interface RuleNode {
	id: string
	name: string
	path: string
	key: string | null
	kind: RuleKind
	rules: Rule[]
	children: RuleNode[]
}

const GLOB = /[*?[]/

function cleanPath(raw: string): string | null {
	const parts: string[] = []
	for (const part of raw.split('/')) {
		const p = part.trim()
		if (!p || p === '.') continue
		if (p === '..') return null
		parts.push(p)
	}
	if (parts.length === 0) return null
	return parts.join('/') + (raw.trimEnd().endsWith('/') ? '/' : '')
}

function ruleLine(path: string, key: string | null, unlock: boolean): string {
	return `${unlock ? '' : '!'}${path}${key ? `#${key}` : ''}`
}

export function parseRule(line: string): Rule | null {
	let text = line.replace(/^﻿/, '').trim()
	if (!text || text.startsWith('#')) return null
	const unlock = !text.startsWith('!')
	if (!unlock) text = text.slice(1).trim()
	text = text.replace(/\\/g, '/')
	const hash = text.indexOf('#')
	const key = hash >= 0 ? text.slice(hash + 1).trim() || null : null
	const path = cleanPath(hash >= 0 ? text.slice(0, hash) : text)
	if (!path) return null
	return { line: ruleLine(path, key, unlock), unlock, path, key }
}

export function fileRule(rules: Rule[], rel: string): Rule | null {
	let found: Rule | null = null
	for (const rule of rules) {
		if (rule.key === null && globMatches(rule.path, rel)) found = rule
	}
	return found
}

export function fileUnlocked(rules: Rule[], rel: string): boolean {
	return fileRule(rules, rel)?.unlock ?? false
}

export function settingUnlocked(rules: Rule[], rel: string, key: string): boolean {
	let state = fileUnlocked(rules, rel)
	for (const rule of rules) {
		if (rule.key === key && globMatches(rule.path, rel)) state = rule.unlock
	}
	return state
}

export function planRule(
	rules: Rule[],
	path: string,
	key: string | null,
	unlock: boolean,
): { remove: string[]; add: string[] } {
	const now = key === null ? fileUnlocked(rules, path) : settingUnlocked(rules, path, key)
	if (now === unlock) return { remove: [], add: [] }
	const exact = rules.filter((r) => r.path === path && r.key === key)
	const rest = rules.filter((r) => !exact.includes(r))
	const state = key === null ? fileUnlocked(rest, path) : settingUnlocked(rest, path, key)
	return {
		remove: exact.map((r) => r.line),
		add: state === unlock ? [] : [ruleLine(path, key, unlock)],
	}
}

function node(
	id: string,
	name: string,
	path: string,
	key: string | null,
	kind: RuleKind,
): RuleNode {
	return { id, name, path, key, kind, rules: [], children: [] }
}

function order(kind: RuleKind): number {
	return { folder: 0, glob: 1, file: 2, setting: 3 }[kind]
}

function finish(item: RuleNode, root: boolean): RuleNode {
	const children = item.children.map((child) => finish(child, false))
	if (!root && item.rules.length === 0 && children.length === 1 && item.kind !== 'setting') {
		const only = children[0]
		return { ...only, name: item.name + (only.kind === 'setting' ? '#' : '') + only.name }
	}
	children.sort((a, b) => order(a.kind) - order(b.kind) || a.name.localeCompare(b.name))
	return { ...item, children }
}

export function ruleTree(rules: Rule[]): RuleNode[] {
	const root = node('', '', '', null, 'folder')
	const nodes = new Map<string, RuleNode>([['folder:', root]])
	const child = (parent: RuleNode, made: RuleNode): RuleNode => {
		const found = nodes.get(made.id)
		if (found) return found
		nodes.set(made.id, made)
		parent.children.push(made)
		return made
	}
	const folder = (path: string): RuleNode => {
		let current = root
		let at = ''
		for (const part of path.split('/').filter(Boolean)) {
			at += `${part}/`
			current = child(current, node(`folder:${at}`, `${part}/`, at, null, 'folder'))
		}
		return current
	}
	const holder = (path: string): RuleNode => {
		if (path.endsWith('/')) return folder(path)
		const literal = GLOB.test(path) ? path.slice(0, path.search(GLOB)) : path
		const cut = literal.lastIndexOf('/') + 1
		const parent = folder(path.slice(0, cut))
		const kind: RuleKind = GLOB.test(path) ? 'glob' : 'file'
		return child(parent, node(`${kind}:${path}`, path.slice(cut), path, null, kind))
	}
	for (const rule of rules) {
		const at = holder(rule.path)
		if (rule.key === null) {
			at.rules.push(rule)
			continue
		}
		const id = `setting:${rule.path}#${rule.key}`
		child(at, node(id, rule.key, rule.path, rule.key, 'setting')).rules.push(rule)
	}
	return finish(root, true).children
}

export function rulesUnder(item: RuleNode): Rule[] {
	return [...item.rules, ...item.children.flatMap(rulesUnder)]
}

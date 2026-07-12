import type { Lang } from '../highlight'

const KEYWORDS: Record<string, string[]> = {
	js: [
		'const',
		'let',
		'var',
		'function',
		'return',
		'if',
		'else',
		'for',
		'while',
		'async',
		'await',
		'import',
		'export',
		'from',
		'class',
		'new',
		'true',
		'false',
		'null',
		'undefined',
		'typeof',
		'this',
		'switch',
		'case',
		'break',
		'continue',
	],
	json: ['true', 'false', 'null'],
	json5: ['true', 'false', 'null', 'Infinity', 'NaN'],
	toml: ['true', 'false'],
	properties: ['true', 'false'],
	css: [],
	text: [],
}

export const PAIRS: Record<string, string> = {
	'{': '}',
	'[': ']',
	'(': ')',
	'"': '"',
	"'": "'",
	'`': '`',
}
const CLOSERS = new Set(Object.values(PAIRS))

export function isCloser(ch: string): boolean {
	return CLOSERS.has(ch)
}

export function completionsFor(code: string, lang: Lang, prefix: string): string[] {
	if (prefix.length < 1) return []
	const set = new Set<string>()
	const words = code.match(/[A-Za-z_$][\w$-]*/g) ?? []
	for (const w of words) set.add(w)
	for (const k of KEYWORDS[lang] ?? []) set.add(k)
	const p = prefix.toLowerCase()
	return [...set]
		.filter((w) => w.length > 1 && w.toLowerCase().startsWith(p) && w !== prefix)
		.sort((a, b) => a.length - b.length || a.localeCompare(b))
		.slice(0, 8)
}

function firstEq(line: string): number {
	let q = ''
	for (let i = 0; i < line.length; i++) {
		const c = line[i]
		if (q) {
			if (c === '\\') i++
			else if (c === q) q = ''
			continue
		}
		if (c === '"' || c === "'") q = c
		else if (c === '=') return i
		else if (c === '#') return -1
	}
	return -1
}

function formatToml(code: string): string {
	const out: string[] = []
	for (const raw of code.split('\n')) {
		const t = raw.trim()
		if (t.startsWith('[')) {
			if (out.length && out[out.length - 1].trim() !== '') out.push('')
			out.push(t)
			continue
		}
		if (t === '' || t.startsWith('#')) {
			out.push(t)
			continue
		}
		const eq = firstEq(t)
		if (eq > 0) out.push(`${t.slice(0, eq).trim()} = ${t.slice(eq + 1).trim()}`)
		else out.push(t)
	}
	return out.join('\n').replace(/\n{3,}/g, '\n\n')
}

function formatProperties(code: string): string {
	return code
		.split('\n')
		.map((raw) => {
			const t = raw.trimEnd()
			const s = t.trim()
			if (s === '' || s.startsWith('#') || s.startsWith('!')) return s
			const eq = firstEq(s)
			if (eq > 0) return `${s.slice(0, eq).trim()}=${s.slice(eq + 1).trim()}`
			return s
		})
		.join('\n')
}

interface LineScan {
	lead: number
	delta: number
	inBlock: boolean
}

function scanLine(line: string, hashComment: boolean, inBlock: boolean): LineScan {
	let i = 0
	let lead = 0
	let delta = 0
	let leading = true
	while (i < line.length) {
		const c = line[i]
		if (inBlock) {
			if (c === '*' && line[i + 1] === '/') {
				inBlock = false
				i += 2
				continue
			}
			i++
			continue
		}
		if (c === '/' && line[i + 1] === '*') {
			inBlock = true
			i += 2
			leading = false
			continue
		}
		if (c === '/' && line[i + 1] === '/') break
		if (hashComment && c === '#') break
		if (c === '"' || c === "'" || c === '`') {
			i++
			while (i < line.length) {
				if (line[i] === '\\') {
					i += 2
					continue
				}
				if (line[i] === c) {
					i++
					break
				}
				i++
			}
			leading = false
			continue
		}
		if (c === '}' || c === ']' || c === ')') {
			delta--
			if (leading) lead++
			i++
			continue
		}
		if (c === '{' || c === '[' || c === '(') {
			delta++
			leading = false
			i++
			continue
		}
		if (!/\s/.test(c)) leading = false
		i++
	}
	return { lead, delta, inBlock }
}

export function formatCode(code: string, lang: Lang): string {
	if (lang === 'json' || lang === 'json5') {
		try {
			return JSON.stringify(JSON.parse(code), null, 2) + '\n'
		} catch {
			void 0
		}
	}
	if (lang === 'toml') return formatToml(code)
	if (lang === 'properties') return formatProperties(code)
	if (lang === 'text') return code

	const lines = code.replace(/\t/g, '  ').split('\n')
	const out: string[] = []
	let depth = 0
	let inBlock = false
	for (const raw of lines) {
		const line = raw.trim()
		if (!line) {
			out.push('')
			continue
		}
		const a = scanLine(line, false, inBlock)
		out.push('  '.repeat(Math.max(0, depth - a.lead)) + line)
		depth = Math.max(0, depth + a.delta)
		inBlock = a.inBlock
	}
	return out.join('\n')
}

export function commentToken(lang: Lang): string {
	switch (lang) {
		case 'js':
		case 'json':
		case 'json5':
		case 'css':
			return '//'
		case 'toml':
		case 'properties':
			return '#'
		default:
			return ''
	}
}

export function indentAfter(prevLine: string): number {
	const t = prevLine.trimEnd()
	const lead = prevLine.length - prevLine.trimStart().length
	let extra = 0
	if (/[{[(]$/.test(t) || /:$/.test(t) || /=\s*$/.test(t)) extra = 2
	return lead + extra
}

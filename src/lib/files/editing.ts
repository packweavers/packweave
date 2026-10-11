import type { Lang } from './highlight'

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

const PAIRS: Record<string, string> = {
	'{': '}',
	'[': ']',
	'(': ')',
	'"': '"',
	"'": "'",
	'`': '`',
}
const CLOSERS = new Set(Object.values(PAIRS))

function isCloser(ch: string): boolean {
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
			if (c === '\\') {
				i++
			} else if (c === q) {
				q = ''
			}
			continue
		}
		if (c === '"' || c === "'") {
			q = c
		} else if (c === '=') {
			return i
		} else if (c === '#') {
			return -1
		}
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
		if (eq > 0) {
			out.push(`${t.slice(0, eq).trim()} = ${t.slice(eq + 1).trim()}`)
		} else {
			out.push(t)
		}
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

function formatJson(code: string): string | null {
	try {
		return JSON.stringify(JSON.parse(code), null, 2) + '\n'
	} catch {
		return null
	}
}

export function formatCode(code: string, lang: Lang): string {
	if (lang === 'json' || lang === 'json5') {
		const formatted = formatJson(code)
		if (formatted !== null) return formatted
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

function commentToken(lang: Lang): string {
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

export interface Edit {
	text: string
	start: number
	end: number
}

function lineRange(text: string, start: number, end: number): { start: number; end: number } {
	const from = text.lastIndexOf('\n', start - 1) + 1
	let to = text.indexOf('\n', end)
	if (to === -1) to = text.length
	return { start: from, end: to }
}

export function moveLines(
	text: string,
	start: number,
	end: number,
	direction: -1 | 1,
): Edit | null {
	const range = lineRange(text, start, end)
	const block = text.slice(range.start, range.end)
	if (direction < 0) {
		if (range.start === 0) return null
		const prevStart = text.lastIndexOf('\n', range.start - 2) + 1
		const prevLine = text.slice(prevStart, range.start - 1)
		const shift = -(range.start - prevStart)
		return {
			text: text.slice(0, prevStart) + block + '\n' + prevLine + text.slice(range.end),
			start: start + shift,
			end: end + shift,
		}
	}
	if (range.end >= text.length) return null
	const nextEndAt = text.indexOf('\n', range.end + 1)
	const nextEnd = nextEndAt === -1 ? text.length : nextEndAt
	const nextLine = text.slice(range.end + 1, nextEnd)
	const shift = nextLine.length + 1
	return {
		text: text.slice(0, range.start) + nextLine + '\n' + block + text.slice(nextEnd),
		start: start + shift,
		end: end + shift,
	}
}

export function duplicateLines(text: string, start: number, end: number): Edit {
	const range = lineRange(text, start, end)
	const block = text.slice(range.start, range.end)
	const shift = block.length + 1
	return {
		text: text.slice(0, range.end) + '\n' + block + text.slice(range.end),
		start: start + shift,
		end: end + shift,
	}
}

export function deleteLines(text: string, start: number, end: number): Edit {
	const range = lineRange(text, start, end)
	let from = range.start
	let to = range.end
	if (to < text.length) {
		to += 1
	} else if (from > 0) {
		from -= 1
	}
	const next = text.slice(0, from) + text.slice(to)
	const at = Math.min(from, next.length)
	return { text: next, start: at, end: at }
}

export function toggleComment(text: string, start: number, end: number, lang: Lang): Edit | null {
	const token = commentToken(lang)
	if (!token) return null
	const range = lineRange(text, start, end)
	const lines = text.slice(range.start, range.end).split('\n')
	const filled = lines.filter((l) => l.trim().length > 0)
	if (filled.length === 0) return null
	const commented = filled.every((l) => l.trimStart().startsWith(token))
	let next: string[]
	if (commented) {
		next = lines.map((l) => {
			const at = l.indexOf(token)
			if (at < 0) return l
			const after = l.slice(at + token.length)
			return l.slice(0, at) + (after.startsWith(' ') ? after.slice(1) : after)
		})
	} else {
		const indent = Math.min(...filled.map((l) => l.length - l.trimStart().length))
		next = lines.map((l) =>
			l.trim().length ? l.slice(0, indent) + token + ' ' + l.slice(indent) : l,
		)
	}
	const block = next.join('\n')
	return {
		text: text.slice(0, range.start) + block + text.slice(range.end),
		start: range.start,
		end: range.start + block.length,
	}
}

export function indentLines(text: string, start: number, end: number, outdent: boolean): Edit {
	if (start === end && !outdent) {
		return { text: text.slice(0, start) + '  ' + text.slice(end), start: start + 2, end: start + 2 }
	}
	const from = text.lastIndexOf('\n', start - 1) + 1
	const block = text.slice(from, end)
	const changed = outdent ? block.replace(/^( {1,2}|\t)/gm, '') : block.replace(/^/gm, '  ')
	return {
		text: text.slice(0, from) + changed + text.slice(end),
		start: outdent ? Math.max(from, start - 2) : start + 2,
		end: from + changed.length,
	}
}

export function newline(text: string, start: number, end: number): Edit {
	const lineStart = text.lastIndexOf('\n', start - 1) + 1
	const line = text.slice(lineStart, start)
	const indent = /^[ \t]*/.exec(line)?.[0] ?? ''
	const before = text[start - 1]
	const after = text[start]
	if ((before === '{' || before === '[' || before === '(') && PAIRS[before] === after) {
		const inner = indent + '  '
		return {
			text: text.slice(0, start) + '\n' + inner + '\n' + indent + text.slice(end),
			start: start + 1 + inner.length,
			end: start + 1 + inner.length,
		}
	}
	const extra = /[{[(:]\s*$|=\s*$/.test(line) ? '  ' : ''
	const insert = '\n' + indent + extra
	return {
		text: text.slice(0, start) + insert + text.slice(end),
		start: start + insert.length,
		end: start + insert.length,
	}
}

export function typePair(text: string, start: number, end: number, key: string): Edit | null {
	if (start === end && isCloser(key) && text[start] === key) {
		return { text, start: start + 1, end: start + 1 }
	}
	const close = PAIRS[key]
	if (!close) return null
	if (start !== end) {
		return {
			text: text.slice(0, start) + key + text.slice(start, end) + close + text.slice(end),
			start: start + 1,
			end: end + 1,
		}
	}
	const quote = key === '"' || key === "'" || key === '`'
	const roomAfter = end >= text.length || /[\s)\]},;:]/.test(text[end] ?? ' ')
	if (quote && !roomAfter) return null
	return {
		text: text.slice(0, start) + key + close + text.slice(end),
		start: start + 1,
		end: start + 1,
	}
}

export function backspace(text: string, start: number, end: number): Edit | null {
	if (start !== end || start === 0) return null
	if (PAIRS[text[start - 1]] === text[start]) {
		return {
			text: text.slice(0, start - 1) + text.slice(start + 1),
			start: start - 1,
			end: start - 1,
		}
	}
	const lineStart = text.lastIndexOf('\n', start - 1) + 1
	const before = text.slice(lineStart, start)
	if (before.length >= 2 && /^ +$/.test(before)) {
		const remove = before.length % 2 === 0 ? 2 : 1
		return {
			text: text.slice(0, start - remove) + text.slice(start),
			start: start - remove,
			end: start - remove,
		}
	}
	return null
}

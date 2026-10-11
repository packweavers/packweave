import type { Lang } from './highlight'

export type ScalarType = 'string' | 'number' | 'bool' | 'raw'

export interface LineEntry {
	line: number
	table: string
	key: string
	type: ScalarType
	value: string | number | boolean
	start: number
	end: number
	comment: string
}

function eqOutside(ln: string): number {
	let q = ''
	for (let i = 0; i < ln.length; i++) {
		const c = ln[i]
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

function propSep(ln: string): number {
	if (/^[A-Za-z]:[^=]*=/.test(ln.trim())) return ln.indexOf('=')
	for (let i = 0; i < ln.length; i++) {
		const c = ln[i]
		if (c === '=' || c === ':') return i
	}
	return -1
}

function unquote(s: string): string {
	if (s.length >= 2 && (s[0] === '"' || s[0] === "'") && s[s.length - 1] === s[0]) {
		return s.slice(1, -1).replace(/\\"/g, '"').replace(/\\\\/g, '\\')
	}
	return s
}

function classify(
	raw: string,
	toml: boolean,
): { type: ScalarType; value: string | number | boolean } {
	if (toml && raw.length >= 2 && (raw[0] === '"' || raw[0] === "'")) {
		return { type: 'string', value: unquote(raw) }
	}
	if (raw === 'true' || raw === 'false') return { type: 'bool', value: raw === 'true' }
	if (/^-?\d+$/.test(raw)) return { type: 'number', value: parseInt(raw, 10) }
	if (/^-?\d*\.\d+$/.test(raw)) return { type: 'number', value: parseFloat(raw) }
	if (toml && (raw.startsWith('[') || raw.startsWith('{'))) return { type: 'raw', value: raw }
	return { type: 'string', value: raw }
}

export function parseConfigLines(
	text: string,
	lang: Lang,
): { lines: string[]; entries: LineEntry[] } {
	const lines = text.split('\n')
	const entries: LineEntry[] = []
	let table = ''
	let pending: string[] = []
	const categories: string[] = []
	const toml = lang === 'toml'
	for (let i = 0; i < lines.length; i++) {
		const ln = lines[i]
		const t = ln.trim()
		if (t === '') {
			pending = []
			continue
		}
		if (t.startsWith('[') && (toml || t.endsWith(']'))) {
			table = t
				.replace(/^\[+/, '')
				.replace(/\]+.*$/, '')
				.trim()
			pending = []
			continue
		}
		if (!toml && t.endsWith('{') && !t.includes('=')) {
			categories.push(
				t
					.slice(0, -1)
					.trim()
					.replace(/^["']|["']$/g, ''),
			)
			table = categories.join('.')
			pending = []
			continue
		}
		if (!toml && t === '}') {
			categories.pop()
			table = categories.join('.')
			pending = []
			continue
		}
		if (t.startsWith('#') || t.startsWith('!') || t.startsWith(';')) {
			pending.push(t.replace(/^[#!;]+\s?/, ''))
			continue
		}
		const sep = toml ? eqOutside(ln) : propSep(ln)
		if (sep < 0) {
			pending = []
			continue
		}
		const key = ln
			.slice(0, sep)
			.trim()
			.replace(/^["']|["']$/g, '')
		if (!key) continue
		let vs = sep + 1
		while (vs < ln.length && (ln[vs] === ' ' || ln[vs] === '\t')) vs++
		let ve: number
		const first = ln[vs]
		if (toml && (first === '"' || first === "'")) {
			ve = vs + 1
			while (ve < ln.length) {
				if (ln[ve] === '\\') {
					ve += 2
					continue
				}
				if (ln[ve] === first) {
					ve++
					break
				}
				ve++
			}
		} else {
			let endLimit = ln.length
			if (toml) {
				let q = ''
				for (let k = vs; k < ln.length; k++) {
					const c = ln[k]
					if (q) {
						if (c === '\\') {
							k++
						} else if (c === q) {
							q = ''
						}
						continue
					}
					if (c === '"' || c === "'") {
						q = c
					} else if (c === '#') {
						endLimit = k
						break
					}
				}
			}
			ve = endLimit
			while (ve > vs && (ln[ve - 1] === ' ' || ln[ve - 1] === '\t' || ln[ve - 1] === '\r')) ve--
		}
		const { type, value } = classify(ln.slice(vs, ve), toml)
		entries.push({
			line: i,
			table,
			key,
			type,
			value,
			start: vs,
			end: ve,
			comment: pending.join('\n'),
		})
		pending = []
	}
	return { lines, entries }
}

function serializeValue(value: string | number | boolean, type: ScalarType, toml: boolean): string {
	if (type === 'bool') return value ? 'true' : 'false'
	if (type === 'number') return String(value)
	if (type === 'raw') return String(value)
	if (toml) return '"' + String(value).replace(/\\/g, '\\\\').replace(/"/g, '\\"') + '"'
	return String(value)
}

export function applyEntry(
	lines: string[],
	lang: Lang,
	entry: LineEntry,
	newValue: string | number | boolean,
): string {
	const serial = serializeValue(newValue, entry.type, lang === 'toml')
	const ln = lines[entry.line]
	const copy = lines.slice()
	copy[entry.line] = ln.slice(0, entry.start) + serial + ln.slice(entry.end)
	return copy.join('\n')
}

export interface JsonLeaf {
	id: string
	type: 'string' | 'number' | 'bool' | 'null'
	value: string | number | boolean | null
	start: number
	end: number
}

function jsonScalar(raw: string): Pick<JsonLeaf, 'type' | 'value'> {
	if (raw === 'true' || raw === 'false') return { type: 'bool', value: raw === 'true' }
	if (raw === 'null') return { type: 'null', value: null }
	const number = Number(raw)
	if (raw !== '' && !Number.isNaN(number)) return { type: 'number', value: number }
	return { type: 'string', value: raw }
}

export function jsonProblem(s: string): string | null {
	let i = 0

	function fail(at: number, message: string): never {
		throw new Error(`Line ${s.slice(0, at).split('\n').length}: ${message}`)
	}

	function skip(): void {
		while (i < s.length) {
			const c = s[i]
			if (/\s/.test(c) || c === '\uFEFF') {
				i++
			} else if (c === '/' && s[i + 1] === '/') {
				while (i < s.length && s[i] !== '\n') i++
			} else if (c === '/' && s[i + 1] === '*') {
				const end = s.indexOf('*/', i + 2)
				if (end < 0) fail(i, 'comment not closed')
				i = end + 2
			} else {
				return
			}
		}
	}

	function string(): void {
		const start = i
		const quote = s[i++]
		while (i < s.length) {
			const c = s[i++]
			if (c === '\\') i++
			else if (c === quote) return
			else if (c === '\n') break
		}
		fail(start, 'string not closed')
	}

	function bare(stop: RegExp): void {
		const start = i
		while (i < s.length && !stop.test(s[i])) {
			if (s[i] === '/' && (s[i + 1] === '/' || s[i + 1] === '*')) break
			i++
		}
		if (i === start) fail(i, `unexpected ${s[i]}`)
	}

	function value(): void {
		skip()
		if (i >= s.length) fail(i, 'value missing')
		const c = s[i]
		if (c === '{') object()
		else if (c === '[') array()
		else if (c === '"' || c === "'") string()
		else bare(/[,}\]\s]/)
	}

	function object(): void {
		const open = i++
		for (;;) {
			skip()
			if (i >= s.length) fail(open, '{ not closed')
			if (s[i] === '}') {
				i++
				return
			}
			if (s[i] === ',') {
				i++
				continue
			}
			if (s[i] === ']') fail(i, 'unexpected ]')
			if (s[i] === '"' || s[i] === "'") string()
			else bare(/[:=,}\s]/)
			skip()
			if (s[i] !== ':' && s[i] !== '=') fail(i, 'missing :')
			i++
			value()
		}
	}

	function array(): void {
		const open = i++
		for (;;) {
			skip()
			if (i >= s.length) fail(open, '[ not closed')
			if (s[i] === ']') {
				i++
				return
			}
			if (s[i] === ',') {
				i++
				continue
			}
			if (s[i] === '}') fail(i, 'unexpected }')
			value()
		}
	}

	try {
		skip()
		if (i >= s.length) return null
		value()
		skip()
		if (i < s.length) fail(i, `unexpected ${s[i]}`)
		return null
	} catch (error) {
		return error instanceof Error ? error.message : String(error)
	}
}

export function jsonLeaves(s: string): JsonLeaf[] {
	const out: JsonLeaf[] = []
	const seen = new Set<string>()
	let i = 0

	function ws(): void {
		while (i < s.length) {
			const c = s[i]
			if (/\s/.test(c) || c === '\uFEFF') {
				i++
			} else if (c === '/' && s[i + 1] === '/') {
				while (i < s.length && s[i] !== '\n') i++
			} else if (c === '/' && s[i + 1] === '*') {
				const end = s.indexOf('*/', i + 2)
				i = end < 0 ? s.length : end + 2
			} else {
				return
			}
		}
	}

	function string(): string {
		const quote = s[i++]
		let text = ''
		while (i < s.length) {
			const c = s[i++]
			if (c === quote) return text
			if (c !== '\\' || i >= s.length) {
				text += c
				continue
			}
			const e = s[i++]
			if (e === 'n') text += '\n'
			else if (e === 't') text += '\t'
			else if (e === 'r') text += '\r'
			else if (e === 'b') text += '\b'
			else if (e === 'f') text += '\f'
			else if (e === 'u' && i + 4 <= s.length) {
				const code = parseInt(s.slice(i, i + 4), 16)
				if (Number.isNaN(code)) {
					text += 'u'
				} else {
					text += String.fromCharCode(code)
					i += 4
				}
			} else text += e
		}
		return text
	}

	function key(): string {
		if (s[i] === '"' || s[i] === "'") return string()
		const start = i
		while (i < s.length && !/[:=,}\s]/.test(s[i])) i++
		return s.slice(start, i)
	}

	function scalar(path: string): void {
		const start = i
		if (s[i] === '"' || s[i] === "'") {
			const text = string()
			leaf(path, { type: 'string', value: text }, start)
			return
		}
		while (i < s.length) {
			const c = s[i]
			if (c === ',' || c === '}' || c === ']' || /\s/.test(c)) break
			if (c === '/' && (s[i + 1] === '/' || s[i + 1] === '*')) break
			i++
		}
		leaf(path, jsonScalar(s.slice(start, i)), start)
	}

	function leaf(path: string, scalar: Pick<JsonLeaf, 'type' | 'value'>, start: number): void {
		if (!path || seen.has(path)) return
		seen.add(path)
		out.push({ id: path, ...scalar, start, end: i })
	}

	function value(path: string): void {
		ws()
		if (i >= s.length) return
		if (s[i] === '{') {
			i++
			object(path)
		} else if (s[i] === '[') {
			i++
			array(path)
		} else {
			scalar(path)
		}
	}

	function object(path: string): void {
		while (i < s.length) {
			ws()
			if (i >= s.length) return
			if (s[i] === '}') {
				i++
				return
			}
			if (s[i] === ',') {
				i++
				continue
			}
			const before = i
			const name = key()
			ws()
			if (s[i] === ':' || s[i] === '=') i++
			value(path ? `${path}.${name}` : name)
			if (i === before) i++
		}
	}

	function array(path: string): void {
		let index = 0
		while (i < s.length) {
			ws()
			if (i >= s.length) return
			if (s[i] === ']') {
				i++
				return
			}
			if (s[i] === ',') {
				i++
				continue
			}
			const before = i
			value(path ? `${path}.${index}` : String(index))
			index++
			if (i === before) i++
		}
	}

	value('')
	return out
}

export function replaceJsonLeaf(
	text: string,
	leaf: JsonLeaf,
	value: string | number | boolean | null,
): string {
	const serial =
		value === null ? 'null' : typeof value === 'string' ? JSON.stringify(value) : String(value)
	return text.slice(0, leaf.start) + serial + text.slice(leaf.end)
}

export function settingKeys(text: string, lang: Lang): string[] {
	if (lang === 'json' || lang === 'json5') return jsonLeaves(text).map((leaf) => leaf.id)
	return parseConfigLines(text, lang).entries.map((e) => (e.table ? `${e.table}.${e.key}` : e.key))
}

export function settingLine(text: string, lang: Lang, key: string): number | null {
	if (lang === 'json' || lang === 'json5') {
		const leaf = jsonLeaves(text).find((l) => l.id === key)
		return leaf ? text.slice(0, leaf.start).split('\n').length : null
	}
	const entry = parseConfigLines(text, lang).entries.find(
		(e) => (e.table ? `${e.table}.${e.key}` : e.key) === key,
	)
	return entry ? entry.line + 1 : null
}

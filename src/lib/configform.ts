import type { Lang } from '../highlight'

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

export type JsonValue = string | number | boolean | null | JsonValue[] | { [k: string]: JsonValue }

export interface JsonLeaf {
	path: string[]
	type: 'string' | 'number' | 'bool' | 'null'
	value: string | number | boolean | null
}

function eqOutside(ln: string): number {
	let q = ''
	for (let i = 0; i < ln.length; i++) {
		const c = ln[i]
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

function propSep(ln: string): number {
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
	const toml = lang === 'toml'
	for (let i = 0; i < lines.length; i++) {
		const ln = lines[i]
		const t = ln.trim()
		if (t === '') {
			pending = []
			continue
		}
		if (toml && t.startsWith('[')) {
			table = t
				.replace(/^\[+/, '')
				.replace(/\]+.*$/, '')
				.trim()
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
						if (c === '\\') k++
						else if (c === q) q = ''
						continue
					}
					if (c === '"' || c === "'") q = c
					else if (c === '#') {
						endLimit = k
						break
					}
				}
			}
			ve = endLimit
			while (ve > vs && (ln[ve - 1] === ' ' || ln[ve - 1] === '\t')) ve--
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

export function serializeValue(
	value: string | number | boolean,
	type: ScalarType,
	toml: boolean,
): string {
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

export function flattenJson(v: JsonValue, path: string[] = [], out: JsonLeaf[] = []): JsonLeaf[] {
	if (v === null) {
		out.push({ path, type: 'null', value: null })
		return out
	}
	if (Array.isArray(v)) {
		v.forEach((x, i) => flattenJson(x, [...path, String(i)], out))
		return out
	}
	if (typeof v === 'object') {
		for (const k of Object.keys(v)) flattenJson(v[k], [...path, k], out)
		return out
	}
	if (typeof v === 'boolean') out.push({ path, type: 'bool', value: v })
	else if (typeof v === 'number') out.push({ path, type: 'number', value: v })
	else out.push({ path, type: 'string', value: v })
	return out
}

export function setByPath(root: JsonValue, path: string[], value: JsonValue): void {
	let o = root as { [k: string]: JsonValue }
	for (let i = 0; i < path.length - 1; i++) {
		o = o[path[i]] as { [k: string]: JsonValue }
	}
	o[path[path.length - 1]] = value
}

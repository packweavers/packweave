export type Lang = 'json' | 'json5' | 'toml' | 'css' | 'js' | 'properties' | 'text'

interface Rule {
	type: string
	re: RegExp
}

export function langFromName(name: string): Lang {
	const lower = name.toLowerCase()
	if (lower.endsWith('.mcmeta')) return 'json'
	const ext = lower.split('.').pop() ?? ''
	switch (ext) {
		case 'json':
			return 'json'
		case 'json5':
			return 'json5'
		case 'toml':
			return 'toml'
		case 'css':
			return 'css'
		case 'js':
		case 'mjs':
		case 'cjs':
			return 'js'
		case 'properties':
		case 'cfg':
		case 'ini':
		case 'conf':
			return 'properties'
		default:
			return 'text'
	}
}

const LINE_SLASH: Rule = { type: 'comment', re: /\/\/[^\n]*/y }
const LINE_HASH: Rule = { type: 'comment', re: /#[^\n]*/y }
const BLOCK: Rule = { type: 'comment', re: /\/\*[\s\S]*?\*\//y }
const STR_D: Rule = { type: 'string', re: /"(?:\\.|[^"\\])*"/y }
const STR_S: Rule = { type: 'string', re: /'(?:\\.|[^'\\])*'/y }
const STR_T: Rule = { type: 'string', re: /`(?:\\.|[^`\\])*`/y }
const NUM: Rule = { type: 'number', re: /-?\b\d[\d_]*(?:\.\d+)?(?:[eE][+-]?\d+)?\b/y }
const PUNCT: Rule = { type: 'punct', re: /[{}[\]():,;=]/y }

const RULES: Record<Lang, Rule[]> = {
	json: [STR_D, NUM, { type: 'bool', re: /\b(?:true|false|null)\b/y }, PUNCT],
	json5: [
		LINE_SLASH,
		BLOCK,
		STR_D,
		STR_S,
		NUM,
		{ type: 'bool', re: /\b(?:true|false|null|Infinity|NaN)\b/y },
		PUNCT,
	],
	toml: [
		LINE_HASH,
		{ type: 'section', re: /^[^\S\n]*\[\[?[^\]\n]*\]\]?/my },
		STR_D,
		STR_S,
		NUM,
		{ type: 'bool', re: /\b(?:true|false)\b/y },
		PUNCT,
	],
	css: [
		BLOCK,
		STR_D,
		STR_S,
		{ type: 'atrule', re: /@[\w-]+/y },
		{
			type: 'number',
			re: /#[0-9a-fA-F]{3,8}\b|-?\b\d[\d.]*(?:px|em|rem|%|vh|vw|s|ms|deg|fr|pt)?\b/y,
		},
		PUNCT,
	],
	js: [
		LINE_SLASH,
		BLOCK,
		STR_D,
		STR_S,
		STR_T,
		NUM,
		{
			type: 'keyword',
			re: /\b(?:const|let|var|function|return|if|else|for|while|do|switch|case|break|continue|new|class|extends|import|export|from|default|typeof|instanceof|in|of|await|async|yield|try|catch|finally|throw|this|super|void|delete)\b/y,
		},
		{ type: 'bool', re: /\b(?:true|false|null|undefined)\b/y },
		PUNCT,
	],
	properties: [
		{ type: 'comment', re: /[#!][^\n]*/y },
		{ type: 'section', re: /^[^\S\n]*\[[^\]\n]*\]/my },
		STR_D,
		STR_S,
		{ type: 'bool', re: /\b(?:true|false)\b/y },
		NUM,
	],
	text: [],
}

function esc(s: string): string {
	return s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;')
}

export function highlight(code: string, lang: Lang): string {
	const rules = RULES[lang]
	if (!rules || rules.length === 0) return esc(code)
	let out = ''
	let i = 0
	const n = code.length
	while (i < n) {
		let matched = false
		for (const r of rules) {
			r.re.lastIndex = i
			const m = r.re.exec(code)
			if (m && m.index === i && m[0].length > 0) {
				out += `<span class="t-${r.type}">${esc(m[0])}</span>`
				i += m[0].length
				matched = true
				break
			}
		}
		if (!matched) {
			out += esc(code[i])
			i++
		}
	}
	return out
}

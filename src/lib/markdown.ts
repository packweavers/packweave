function esc(s: string): string {
	return s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;')
}

function inline(raw: string): string {
	return raw
		.split(/(`[^`]+`)/g)
		.map((part) => {
			if (part.length >= 2 && part.startsWith('`') && part.endsWith('`')) {
				return `<code>${esc(part.slice(1, -1))}</code>`
			}
			let s = esc(part)
			s = s.replace(
				/!\[([^\]]*)\]\(([^)\s]+)(?:\s+"[^"]*")?\)/g,
				(_m, alt: string, url: string) => `<img src="${url}" alt="${alt}" />`,
			)
			s = s.replace(
				/\[([^\]]+)\]\(([^)\s]+)(?:\s+"[^"]*")?\)/g,
				(_m, t: string, url: string) => `<a href="${url}">${t}</a>`,
			)
			s = s.replace(/\*\*([^*]+)\*\*/g, '<strong>$1</strong>')
			s = s.replace(/__([^_]+)__/g, '<strong>$1</strong>')
			s = s.replace(/(^|[^*])\*([^*\n]+)\*/g, '$1<em>$2</em>')
			s = s.replace(/(^|[^_])_([^_\n]+)_/g, '$1<em>$2</em>')
			s = s.replace(/~~([^~]+)~~/g, '<del>$1</del>')
			return s
		})
		.join('')
}

function splitRow(line: string): string[] {
	return line
		.replace(/^\s*\|/, '')
		.replace(/\|\s*$/, '')
		.split('|')
		.map((c) => c.trim())
}

export function renderMarkdown(src: string): string {
	const lines = (src ?? '').replace(/\r\n?/g, '\n').split('\n')
	let html = ''
	let i = 0
	const isList = (l: string) => /^(\s*)([-*+])\s+/.test(l) || /^(\s*)\d+[.)]\s+/.test(l)
	while (i < lines.length) {
		const line = lines[i]

		const fence = line.match(/^```(\w*)\s*$/)
		if (fence) {
			const buf: string[] = []
			i++
			while (i < lines.length && !/^```/.test(lines[i])) {
				buf.push(lines[i])
				i++
			}
			i++
			const cls = fence[1] ? ` class="language-${fence[1]}"` : ''
			html += `<pre><code${cls}>${esc(buf.join('\n'))}</code></pre>`
			continue
		}

		if (/^\s*$/.test(line)) {
			i++
			continue
		}

		const h = line.match(/^(#{1,6})\s+(.*)$/)
		if (h) {
			html += `<h${h[1].length}>${inline(h[2].trim())}</h${h[1].length}>`
			i++
			continue
		}

		if (/^\s*([-*_])\s*(\1\s*){2,}$/.test(line)) {
			html += '<hr />'
			i++
			continue
		}

		if (/^\s*>/.test(line)) {
			const buf: string[] = []
			while (i < lines.length && /^\s*>/.test(lines[i])) {
				buf.push(lines[i].replace(/^\s*>\s?/, ''))
				i++
			}
			html += `<blockquote>${renderMarkdown(buf.join('\n'))}</blockquote>`
			continue
		}

		if (
			line.includes('|') &&
			i + 1 < lines.length &&
			/^\s*\|?\s*:?-+:?\s*(\|\s*:?-+:?\s*)+\|?\s*$/.test(lines[i + 1])
		) {
			const header = splitRow(line)
			i += 2
			const rows: string[][] = []
			while (i < lines.length && lines[i].includes('|') && lines[i].trim() !== '') {
				rows.push(splitRow(lines[i]))
				i++
			}
			html += '<table><thead><tr>'
			html += header.map((c) => `<th>${inline(c)}</th>`).join('')
			html += '</tr></thead><tbody>'
			for (const r of rows)
				html += '<tr>' + r.map((c) => `<td>${inline(c)}</td>`).join('') + '</tr>'
			html += '</tbody></table>'
			continue
		}

		if (isList(line)) {
			const ordered = /^(\s*)\d+[.)]\s+/.test(line)
			const items: string[] = []
			while (i < lines.length) {
				if (!isList(lines[i])) {
					if (items.length && /^\s+\S/.test(lines[i])) {
						items[items.length - 1] += '\n' + lines[i].replace(/^\s+/, '')
						i++
						continue
					}
					break
				}
				if (ordered !== /^(\s*)\d+[.)]\s+/.test(lines[i])) break
				const m = lines[i].match(/^(?:\s*)(?:[-*+]|\d+[.)])\s+([\s\S]*)$/)
				items.push(m ? m[1] : lines[i])
				i++
			}
			const tag = ordered ? 'ol' : 'ul'
			html += `<${tag}>`
			for (const it of items) {
				const task = it.match(/^\[([ xX])\]\s+([\s\S]*)$/)
				if (task) {
					const checked = task[1].toLowerCase() === 'x' ? ' checked' : ''
					html += `<li class="task"><input type="checkbox" disabled${checked} /> ${inline(task[2])}</li>`
				} else {
					html += `<li>${inline(it)}</li>`
				}
			}
			html += `</${tag}>`
			continue
		}

		const buf: string[] = []
		while (
			i < lines.length &&
			!/^\s*$/.test(lines[i]) &&
			!/^(#{1,6})\s/.test(lines[i]) &&
			!/^\s*>/.test(lines[i]) &&
			!/^```/.test(lines[i]) &&
			!isList(lines[i])
		) {
			buf.push(lines[i])
			i++
		}
		html += `<p>${inline(buf.join('\n')).replace(/\n/g, '<br />')}</p>`
	}
	return html
}

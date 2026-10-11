import DOMPurify from 'dompurify'
import { Marked } from 'marked'

const parser = new Marked({ gfm: true, breaks: true, async: false })

const TAGS = [
	'a',
	'abbr',
	'b',
	'blockquote',
	'br',
	'caption',
	'center',
	'code',
	'col',
	'colgroup',
	'dd',
	'del',
	'details',
	'div',
	'dl',
	'dt',
	'em',
	'figcaption',
	'figure',
	'h1',
	'h2',
	'h3',
	'h4',
	'h5',
	'h6',
	'hr',
	'i',
	'img',
	'input',
	'ins',
	'kbd',
	'li',
	'mark',
	'ol',
	'p',
	'pre',
	'q',
	's',
	'samp',
	'small',
	'span',
	'strike',
	'strong',
	'sub',
	'summary',
	'sup',
	'table',
	'tbody',
	'td',
	'tfoot',
	'th',
	'thead',
	'tr',
	'u',
	'ul',
]

const ATTRS = [
	'align',
	'alt',
	'checked',
	'colspan',
	'disabled',
	'height',
	'href',
	'open',
	'rowspan',
	'span',
	'src',
	'start',
	'title',
	'type',
	'width',
]

let hooked = false

function hook(): void {
	if (hooked) return
	hooked = true
	DOMPurify.addHook('afterSanitizeAttributes', (node) => {
		const tag = node.tagName.toLowerCase()
		if (tag === 'a') {
			const href = node.getAttribute('href') ?? ''
			if (!/^(https?:|mailto:)/i.test(href)) node.removeAttribute('href')
		}
		if (tag === 'img') {
			const src = node.getAttribute('src') ?? ''
			if (!/^https:/i.test(src)) node.removeAttribute('src')
			node.setAttribute('loading', 'lazy')
			node.setAttribute('referrerpolicy', 'no-referrer')
		}
		if (tag === 'input') {
			if (node.getAttribute('type') !== 'checkbox') {
				node.remove()
				return
			}
			node.setAttribute('disabled', '')
		}
	})
}

function clean(html: string): string {
	hook()
	return DOMPurify.sanitize(html, {
		ALLOWED_TAGS: TAGS,
		ALLOWED_ATTR: ATTRS,
		ALLOW_DATA_ATTR: false,
		ALLOW_ARIA_ATTR: false,
	})
}

export function renderMarkdown(src: string): string {
	return clean(parser.parse((src ?? '').replace(/\r\n?/g, '\n')) as string)
}

export function renderHtml(src: string): string {
	return clean(src ?? '')
}

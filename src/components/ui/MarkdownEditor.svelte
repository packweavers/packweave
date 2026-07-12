<script lang="ts">
	import {
		Bold,
		Italic,
		Strikethrough,
		Heading,
		Quote,
		Code,
		Code2,
		Link,
		Image,
		List,
		ListOrdered,
		ListChecks,
		Table,
		Minus,
		Eye,
		PenLine,
	} from '@lucide/svelte'
	import { renderMarkdown } from '../../lib/markdown'
	import { openExternal } from '../../api'

	let {
		value = $bindable(''),
		placeholder = '',
		element = $bindable<HTMLTextAreaElement | null>(null),
	}: { value?: string; placeholder?: string; element?: HTMLTextAreaElement | null } = $props()

	let ta = $state<HTMLTextAreaElement | null>(null)

	$effect(() => {
		element = ta
	})
	let mode = $state<'write' | 'preview'>('write')
	const html = $derived(renderMarkdown(value ?? ''))

	function edit(fn: (sel: string, before: string, after: string) => { text: string; start: number; end: number }) {
		const el = ta
		if (!el) return
		const v = value ?? ''
		const s = el.selectionStart
		const e = el.selectionEnd
		const r = fn(v.slice(s, e), v.slice(0, s), v.slice(e))
		value = r.text
		requestAnimationFrame(() => {
			el.focus()
			el.selectionStart = r.start
			el.selectionEnd = r.end
		})
	}

	function wrap(token: string, ph = 'text') {
		edit((sel, before, after) => {
			const inner = sel || ph
			const start = before.length + token.length
			return { text: before + token + inner + token + after, start, end: start + inner.length }
		})
	}

	function prefixLines(prefix: string | ((i: number) => string)) {
		edit((sel, before, after) => {
			const lines = (sel || 'text').split('\n')
			const out = lines.map((l, i) => (typeof prefix === 'function' ? prefix(i) : prefix) + l).join('\n')
			const pre = before && !before.endsWith('\n') ? before + '\n' : before
			return { text: pre + out + after, start: pre.length, end: pre.length + out.length }
		})
	}

	function insert(snippet: string) {
		edit((_sel, before, after) => {
			const pre = before && !before.endsWith('\n') ? before + '\n' : before
			const pos = pre.length + snippet.length
			return { text: pre + snippet + after, start: pos, end: pos }
		})
	}

	function link() {
		edit((sel, before, after) => {
			const t = sel || 'text'
			const start = before.length + t.length + 3
			return { text: `${before}[${t}](url)${after}`, start, end: start + 3 }
		})
	}
	function image() {
		edit((sel, before, after) => {
			const t = sel || 'alt'
			const start = before.length + t.length + 4
			return { text: `${before}![${t}](url)${after}`, start, end: start + 3 }
		})
	}
	function codeBlock() {
		edit((sel, before, after) => {
			const inner = sel || 'code'
			const pre = before && !before.endsWith('\n') ? before + '\n' : before
			const start = pre.length + 4
			return { text: pre + '```\n' + inner + '\n```' + after, start, end: start + inner.length }
		})
	}

	const TOOLBAR = [
		{ icon: Bold, title: 'Bold  ⌘B', run: () => wrap('**') },
		{ icon: Italic, title: 'Italic  ⌘I', run: () => wrap('*') },
		{ icon: Strikethrough, title: 'Strikethrough', run: () => wrap('~~') },
		{ icon: Heading, title: 'Heading', run: () => prefixLines('## ') },
		{ icon: Quote, title: 'Quote', run: () => prefixLines('> ') },
		{ icon: Code, title: 'Inline code  ⌘E', run: () => wrap('`') },
		{ icon: Code2, title: 'Code block', run: codeBlock },
		{ icon: Link, title: 'Link  ⌘K', run: link },
		{ icon: Image, title: 'Image', run: image },
		{ icon: List, title: 'Bulleted list', run: () => prefixLines('- ') },
		{ icon: ListOrdered, title: 'Numbered list', run: () => prefixLines((i) => `${i + 1}. `) },
		{ icon: ListChecks, title: 'Task list', run: () => prefixLines('- [ ] ') },
		{ icon: Table, title: 'Table', run: () => insert('| Column | Column |\n| --- | --- |\n| Cell | Cell |\n') },
		{ icon: Minus, title: 'Divider', run: () => insert('\n---\n') },
	]

	function onKeydown(e: KeyboardEvent) {
		const mod = e.metaKey || e.ctrlKey
		const k = e.key.toLowerCase()
		if (mod && k === 'b') return (e.preventDefault(), wrap('**'))
		if (mod && k === 'i') return (e.preventDefault(), wrap('*'))
		if (mod && k === 'k') return (e.preventDefault(), link())
		if (mod && k === 'e') return (e.preventDefault(), wrap('`'))
		const el = e.target as HTMLTextAreaElement
		const v = value ?? ''
		if (e.key === 'Tab') {
			e.preventDefault()
			const s = el.selectionStart
			value = v.slice(0, s) + '  ' + v.slice(el.selectionEnd)
			requestAnimationFrame(() => (el.selectionStart = el.selectionEnd = s + 2))
			return
		}
		if (e.key === 'Enter') {
			const s = el.selectionStart
			const lineStart = v.lastIndexOf('\n', s - 1) + 1
			const lineText = v.slice(lineStart, s)
			const m = lineText.match(/^(\s*)([-*+]\s+\[[ xX]\]\s+|[-*+]\s+|\d+[.)]\s+)/)
			if (!m) return
			e.preventDefault()
			if (lineText.trimEnd() === (m[1] + m[2]).trimEnd()) {
				value = v.slice(0, lineStart) + v.slice(el.selectionEnd)
				requestAnimationFrame(() => (el.selectionStart = el.selectionEnd = lineStart))
				return
			}
			let marker = m[2].replace(/\[[xX]\]/, '[ ]')
			const om = marker.match(/^(\d+)([.)])(\s+)$/)
			if (om) marker = `${parseInt(om[1]) + 1}${om[2]}${om[3]}`
			const ins = '\n' + m[1] + marker
			value = v.slice(0, s) + ins + v.slice(el.selectionEnd)
			requestAnimationFrame(() => (el.selectionStart = el.selectionEnd = s + ins.length))
		}
	}

	function onPreviewClick(e: MouseEvent) {
		const a = (e.target as HTMLElement).closest('a')
		if (a) {
			e.preventDefault()
			const href = a.getAttribute('href')
			if (href && /^https?:/.test(href)) void openExternal(href)
		}
	}

	const tbBtn =
		'grid place-items-center w-7 h-7 rounded-sm text-secondary bg-transparent border-none cursor-pointer hover:bg-button-bg hover:text-contrast'
	const tabBtn =
		'inline-flex items-center gap-1 px-2 h-7 rounded-sm text-[0.74rem] font-[550] text-secondary bg-transparent border-none cursor-pointer hover:text-contrast'
	const prose =
		'text-[0.85rem] leading-[1.6] text-body [&_h1]:text-[1.5rem] [&_h1]:font-bold [&_h1]:mt-3 [&_h1]:mb-2 [&_h2]:text-[1.2rem] [&_h2]:font-bold [&_h2]:mt-3 [&_h2]:mb-2 [&_h3]:text-[1.05rem] [&_h3]:font-semibold [&_h3]:mt-2 [&_h3]:mb-1.5 [&_h4]:font-semibold [&_h4]:mt-2 [&_h4]:mb-1 [&_p]:my-2 [&_ul]:my-2 [&_ul]:pl-5 [&_ul]:list-disc [&_ol]:my-2 [&_ol]:pl-5 [&_ol]:list-decimal [&_li]:my-0.5 [&_li.task]:list-none [&_li.task]:-ml-5 [&_a]:text-link [&_a]:underline [&_strong]:text-contrast [&_code]:font-mono [&_code]:text-[0.85em] [&_code]:bg-button-bg [&_code]:px-1 [&_code]:py-0.5 [&_code]:rounded-sm [&_pre]:bg-bg-inset [&_pre]:border [&_pre]:border-divider [&_pre]:rounded-md [&_pre]:p-2.5 [&_pre]:overflow-x-auto [&_pre]:my-2 [&_pre_code]:bg-transparent [&_pre_code]:p-0 [&_blockquote]:border-l-2 [&_blockquote]:border-divider-dark [&_blockquote]:pl-3 [&_blockquote]:text-secondary [&_blockquote]:my-2 [&_hr]:border-divider [&_hr]:my-3 [&_table]:border-collapse [&_table]:my-2 [&_table]:text-[0.8rem] [&_th]:border [&_th]:border-divider [&_th]:px-2 [&_th]:py-1 [&_th]:bg-bg-raised [&_td]:border [&_td]:border-divider [&_td]:px-2 [&_td]:py-1 [&_img]:max-w-full [&_img]:rounded-md'
</script>

<div
	class="flex flex-col min-h-0 flex-1 border border-divider rounded-md overflow-hidden bg-bg-inset"
>
	<div
		class="flex items-center gap-0.5 flex-wrap px-1.5 py-1 border-b border-divider bg-bg-raised shrink-0"
	>
		{#each TOOLBAR as b (b.title)}
			{@const Icon = b.icon}
			<button type="button" class={tbBtn} title={b.title} onclick={b.run}><Icon size={15} /></button>
		{/each}
		<div class="ml-auto flex items-center gap-0.5">
			<button
				type="button"
				class="{tabBtn} {mode === 'write' ? 'bg-button-bg text-contrast' : ''}"
				onclick={() => (mode = 'write')}><PenLine size={13} /> Write</button
			>
			<button
				type="button"
				class="{tabBtn} {mode === 'preview' ? 'bg-button-bg text-contrast' : ''}"
				onclick={() => (mode = 'preview')}><Eye size={13} /> Preview</button
			>
		</div>
	</div>

	{#if mode === 'write'}
		<textarea
			bind:this={ta}
			bind:value
			{placeholder}
			spellcheck="false"
			onkeydown={onKeydown}
			class="flex-1 min-h-[10rem] w-full resize-none border-0 outline-none bg-bg-inset text-body p-3 font-mono text-[0.82rem] leading-[1.6]"
		></textarea>
	{:else}
		<!-- svelte-ignore a11y_click_events_have_key_events -->
		<!-- svelte-ignore a11y_no_static_element_interactions -->
		<div class="flex-1 min-h-[10rem] overflow-y-auto p-3 {prose}" onclick={onPreviewClick}>
			{@html html}
		</div>
	{/if}
</div>

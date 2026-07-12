<script lang="ts">
	import { highlight, type Lang } from '../../highlight'
	import { PAIRS, isCloser, completionsFor, formatCode, commentToken } from '../../lib/codeedit'

	let {
		value = $bindable(''),
		lang,
		element = $bindable<HTMLTextAreaElement | null>(null),
		onsave,
	}: {
		value?: string
		lang: Lang
		element?: HTMLTextAreaElement | null
		onsave?: () => void
	} = $props()

	let taRef = $state<HTMLTextAreaElement | null>(null)
	let preRef = $state<HTMLPreElement | null>(null)
	let gutterRef = $state<HTMLDivElement | null>(null)

	let suggestions = $state<string[]>([])
	let sugIndex = $state(0)
	let sugTop = $state(0)
	let sugLeft = $state(0)
	let charW = 0
	let lineH = 0

	$effect(() => {
		element = taRef
	})

	const rendered = $derived(highlight(value ?? '', lang) + '\n')
	const lineCount = $derived((value ?? '').split('\n').length)

	$effect(() => {
		void value
		syncOverlays()
	})

	function syncOverlays() {
		const ta = taRef
		if (!ta) return
		if (preRef) {
			preRef.scrollTop = ta.scrollTop
			preRef.scrollLeft = ta.scrollLeft
		}
		if (gutterRef) gutterRef.scrollTop = ta.scrollTop
	}

	function onInput(e: Event) {
		value = (e.target as HTMLTextAreaElement).value
		queueSuggest()
	}

	function syncScroll() {
		syncOverlays()
		if (suggestions.length) queueSuggest()
	}

	function applyEdit(next: string, selStart: number, selEnd: number = selStart) {
		const ta = taRef
		if (!ta) {
			value = next
			return
		}
		ta.value = next
		ta.setSelectionRange(selStart, selEnd)
		value = next
		suggestions = []
		syncOverlays()
	}

	interface Snap {
		value: string
		start: number
		end: number
	}
	let undoStack: Snap[] = []
	let redoStack: Snap[] = []
	let lastEdit = 0
	let grouping = false

	function snapNow(ta: HTMLTextAreaElement): Snap {
		return { value: value ?? '', start: ta.selectionStart, end: ta.selectionEnd }
	}
	function recordStructural(ta: HTMLTextAreaElement) {
		undoStack.push(snapNow(ta))
		if (undoStack.length > 500) undoStack.shift()
		redoStack = []
		grouping = false
	}
	function recordTyping(ta: HTMLTextAreaElement) {
		const now = performance.now()
		if (!grouping || now - lastEdit > 400) {
			undoStack.push(snapNow(ta))
			if (undoStack.length > 500) undoStack.shift()
			redoStack = []
			grouping = true
		}
		lastEdit = now
	}
	function undo() {
		const ta = taRef
		if (!ta || !undoStack.length) return
		redoStack.push(snapNow(ta))
		grouping = false
		const snap = undoStack.pop()!
		applyEdit(snap.value, snap.start, snap.end)
	}
	function redo() {
		const ta = taRef
		if (!ta || !redoStack.length) return
		undoStack.push(snapNow(ta))
		grouping = false
		const snap = redoStack.pop()!
		applyEdit(snap.value, snap.start, snap.end)
	}
	export function format() {
		const ta = taRef
		if (!ta || lang === 'text') return
		recordStructural(ta)
		applyEdit(formatCode(value ?? '', lang), 0)
		ta.scrollTop = 0
		syncOverlays()
	}

	function lineRange(v: string, s: number, en: number): { start: number; end: number } {
		const start = v.lastIndexOf('\n', s - 1) + 1
		let end = v.indexOf('\n', en)
		if (end === -1) end = v.length
		return { start, end }
	}

	function moveLines(ta: HTMLTextAreaElement, dir: number) {
		const v = value ?? ''
		const s = ta.selectionStart
		const en = ta.selectionEnd
		const { start, end } = lineRange(v, s, en)
		const block = v.slice(start, end)
		if (dir < 0) {
			if (start === 0) return
			const prevStart = v.lastIndexOf('\n', start - 2) + 1
			const prevLine = v.slice(prevStart, start - 1)
			recordStructural(ta)
			const next = v.slice(0, prevStart) + block + '\n' + prevLine + v.slice(end)
			const delta = -(start - prevStart)
			applyEdit(next, s + delta, en + delta)
		} else {
			if (end >= v.length) return
			const nextEnd0 = v.indexOf('\n', end + 1)
			const nextEnd = nextEnd0 === -1 ? v.length : nextEnd0
			const nextLine = v.slice(end + 1, nextEnd)
			recordStructural(ta)
			const next = v.slice(0, start) + nextLine + '\n' + block + v.slice(nextEnd)
			const delta = nextLine.length + 1
			applyEdit(next, s + delta, en + delta)
		}
	}

	function duplicateLines(ta: HTMLTextAreaElement) {
		const v = value ?? ''
		const s = ta.selectionStart
		const en = ta.selectionEnd
		const { start, end } = lineRange(v, s, en)
		const block = v.slice(start, end)
		recordStructural(ta)
		const next = v.slice(0, end) + '\n' + block + v.slice(end)
		const delta = block.length + 1
		applyEdit(next, s + delta, en + delta)
	}

	function deleteLines(ta: HTMLTextAreaElement) {
		const v = value ?? ''
		const s = ta.selectionStart
		const en = ta.selectionEnd
		const { start, end } = lineRange(v, s, en)
		let delStart = start
		let delEnd = end
		if (end < v.length) delEnd = end + 1
		else if (start > 0) delStart = start - 1
		recordStructural(ta)
		const next = v.slice(0, delStart) + v.slice(delEnd)
		applyEdit(next, Math.min(delStart, next.length))
	}

	function toggleComment(ta: HTMLTextAreaElement) {
		const token = commentToken(lang)
		if (!token) return
		const v = value ?? ''
		const s = ta.selectionStart
		const en = ta.selectionEnd
		const { start, end } = lineRange(v, s, en)
		const lines = v.slice(start, end).split('\n')
		const nonEmpty = lines.filter((l) => l.trim().length)
		if (!nonEmpty.length) return
		recordStructural(ta)
		const allCommented = nonEmpty.every((l) => l.trimStart().startsWith(token))
		let newLines: string[]
		if (allCommented) {
			newLines = lines.map((l) => {
				const i = l.indexOf(token)
				if (i < 0) return l
				const after = l.slice(i + token.length)
				return l.slice(0, i) + (after.startsWith(' ') ? after.slice(1) : after)
			})
		} else {
			const indent = Math.min(...nonEmpty.map((l) => l.length - l.trimStart().length))
			newLines = lines.map((l) =>
				l.trim().length ? l.slice(0, indent) + token + ' ' + l.slice(indent) : l,
			)
		}
		const newBlock = newLines.join('\n')
		applyEdit(v.slice(0, start) + newBlock + v.slice(end), start, start + newBlock.length)
	}

	function measure() {
		if (!taRef) return
		const cs = getComputedStyle(taRef)
		lineH = parseFloat(cs.lineHeight) || 18
		const span = document.createElement('span')
		span.style.font = cs.font
		span.style.position = 'absolute'
		span.style.visibility = 'hidden'
		span.style.whiteSpace = 'pre'
		span.textContent = 'MMMMMMMMMM'
		document.body.appendChild(span)
		charW = span.getBoundingClientRect().width / 10
		span.remove()
	}

	function currentWord(): { word: string; start: number } {
		const ta = taRef
		if (!ta) return { word: '', start: 0 }
		const pos = ta.selectionStart
		const before = (value ?? '').slice(0, pos)
		const m = before.match(/[A-Za-z_$][\w$-]*$/)
		return { word: m ? m[0] : '', start: m ? pos - m[0].length : pos }
	}

	function queueSuggest() {
		const ta = taRef
		if (!ta || lang === 'text' || ta.selectionStart !== ta.selectionEnd) {
			suggestions = []
			return
		}
		const { word, start } = currentWord()
		if (word.length < 2) {
			suggestions = []
			return
		}
		const list = completionsFor(value ?? '', lang, word)
		suggestions = list
		sugIndex = 0
		if (list.length) {
			if (!charW || !lineH) measure()
			const text = (value ?? '').slice(0, start)
			const lines = text.split('\n')
			const row = lines.length - 1
			const col = lines[lines.length - 1].length
			const pad = 0.7 * 16
			sugTop = pad + (row + 1) * lineH - ta.scrollTop
			sugLeft = pad + col * charW - ta.scrollLeft
		}
	}

	function applyCompletion(word: string) {
		const ta = taRef
		if (!ta) return
		const { start } = currentWord()
		const pos = ta.selectionStart
		applyEdit((value ?? '').slice(0, start) + word + (value ?? '').slice(pos), start + word.length)
	}

	function onKeydown(e: KeyboardEvent) {
		const ta = e.target as HTMLTextAreaElement
		const v = value ?? ''
		const s = ta.selectionStart
		const en = ta.selectionEnd
		const mod = e.metaKey || e.ctrlKey

		if (mod && !e.altKey && (e.key === 'z' || e.key === 'Z') && !e.shiftKey) {
			e.preventDefault()
			undo()
			return
		}
		if (
			mod &&
			!e.altKey &&
			((e.shiftKey && (e.key === 'z' || e.key === 'Z')) || e.key === 'y' || e.key === 'Y')
		) {
			e.preventDefault()
			redo()
			return
		}
		if (mod && !e.altKey && e.key === '/') {
			e.preventDefault()
			toggleComment(ta)
			return
		}
		if (e.altKey && !mod && (e.key === 'ArrowUp' || e.key === 'ArrowDown')) {
			e.preventDefault()
			moveLines(ta, e.key === 'ArrowUp' ? -1 : 1)
			return
		}
		if (mod && e.shiftKey && (e.key === 'd' || e.key === 'D')) {
			e.preventDefault()
			duplicateLines(ta)
			return
		}
		if (mod && e.shiftKey && (e.key === 'k' || e.key === 'K')) {
			e.preventDefault()
			deleteLines(ta)
			return
		}

		if (suggestions.length) {
			if (e.key === 'ArrowDown') {
				e.preventDefault()
				sugIndex = (sugIndex + 1) % suggestions.length
				return
			}
			if (e.key === 'ArrowUp') {
				e.preventDefault()
				sugIndex = (sugIndex - 1 + suggestions.length) % suggestions.length
				return
			}
			if (e.key === 'Enter' || e.key === 'Tab') {
				e.preventDefault()
				applyCompletion(suggestions[sugIndex])
				return
			}
			if (e.key === 'Escape') {
				e.preventDefault()
				suggestions = []
				return
			}
		}

		if (
			e.key.startsWith('Arrow') ||
			e.key === 'Home' ||
			e.key === 'End' ||
			e.key === 'PageUp' ||
			e.key === 'PageDown'
		) {
			grouping = false
			return
		}

		if (s === en && isCloser(e.key) && v[s] === e.key) {
			e.preventDefault()
			recordStructural(ta)
			applyEdit(v, s + 1)
			return
		}

		if (PAIRS[e.key]) {
			const close = PAIRS[e.key]
			const isQuote = e.key === '"' || e.key === "'" || e.key === '`'
			const nextOk = en >= v.length || /[\s)\]},;:]/.test(v[en] ?? ' ')
			if (s !== en) {
				e.preventDefault()
				recordStructural(ta)
				applyEdit(v.slice(0, s) + e.key + v.slice(s, en) + close + v.slice(en), s + 1, en + 1)
				return
			}
			if (!isQuote || nextOk) {
				e.preventDefault()
				recordStructural(ta)
				applyEdit(v.slice(0, s) + e.key + close + v.slice(en), s + 1)
				return
			}
		}

		if (e.key === 'Backspace' && s === en && s > 0 && PAIRS[v[s - 1]] === v[s]) {
			e.preventDefault()
			recordStructural(ta)
			applyEdit(v.slice(0, s - 1) + v.slice(s + 1), s - 1)
			return
		}

		if (e.key === 'Backspace' && s === en && s > 0) {
			const lineStart = v.lastIndexOf('\n', s - 1) + 1
			const before = v.slice(lineStart, s)
			if (before.length >= 2 && /^ +$/.test(before)) {
				const remove = before.length % 2 === 0 ? 2 : 1
				e.preventDefault()
				recordStructural(ta)
				applyEdit(v.slice(0, s - remove) + v.slice(s), s - remove)
				return
			}
		}

		if (e.key === 'Enter') {
			e.preventDefault()
			recordStructural(ta)
			const lineStart = v.lastIndexOf('\n', s - 1) + 1
			const line = v.slice(lineStart, s)
			const indent = line.match(/^[ \t]*/)?.[0] ?? ''
			const prevCh = v[s - 1]
			const nextCh = v[s]
			if ((prevCh === '{' || prevCh === '[' || prevCh === '(') && PAIRS[prevCh] === nextCh) {
				const inner = indent + '  '
				applyEdit(v.slice(0, s) + '\n' + inner + '\n' + indent + v.slice(en), s + 1 + inner.length)
			} else {
				const extra = /[{[(:]\s*$|=\s*$/.test(line) ? '  ' : ''
				const insert = '\n' + indent + extra
				applyEdit(v.slice(0, s) + insert + v.slice(en), s + insert.length)
			}
			return
		}

		if (e.key === 'Tab') {
			e.preventDefault()
			recordStructural(ta)
			if (s !== en || e.shiftKey) {
				const blockStart = v.lastIndexOf('\n', s - 1) + 1
				const seg = v.slice(blockStart, en)
				if (e.shiftKey) {
					const ded = seg.replace(/^( {1,2}|\t)/gm, '')
					applyEdit(
						v.slice(0, blockStart) + ded + v.slice(en),
						Math.max(blockStart, s - 2),
						blockStart + ded.length,
					)
				} else {
					const ind = seg.replace(/^/gm, '  ')
					applyEdit(v.slice(0, blockStart) + ind + v.slice(en), s + 2, blockStart + ind.length)
				}
			} else {
				applyEdit(v.slice(0, s) + '  ' + v.slice(en), s + 2)
			}
			return
		}

		const contentKey =
			(e.key.length === 1 && !mod && !e.altKey) || e.key === 'Backspace' || e.key === 'Delete'
		if (contentKey) recordTyping(ta)
	}

	function onBlur() {
		suggestions = []
		onsave?.()
	}
</script>

<div class="relative flex-1 min-h-0 overflow-hidden flex bg-bg-inset">
	<div
		bind:this={gutterRef}
		aria-hidden="true"
		class="flex-shrink-0 overflow-hidden select-none box-border text-right text-secondary font-mono text-[0.8rem] leading-[1.5] [tab-size:2] pt-[0.7rem] pb-[0.7rem] px-2 border-r border-divider"
		style="width: calc({String(lineCount).length}ch + 1.4rem)"
	>
		{#each Array.from({ length: lineCount }, (_, i) => i + 1) as n (n)}
			<div>{n}</div>
		{/each}
	</div>
	<div class="relative flex-1 min-h-0 overflow-hidden">
		<pre
			bind:this={preRef}
			aria-hidden="true"
			class="absolute inset-0 m-0 p-[0.7rem] overflow-hidden font-mono text-[0.8rem] leading-[1.5] whitespace-pre border-0 pointer-events-none bg-bg-inset text-body [tab-size:2] [&_.t-comment]:text-secondary [&_.t-comment]:italic [&_.t-string]:text-green [&_.t-number]:text-orange [&_.t-bool]:text-blue [&_.t-keyword]:text-blue [&_.t-keyword]:font-semibold [&_.t-atrule]:text-blue [&_.t-atrule]:font-semibold [&_.t-section]:text-blue [&_.t-section]:font-semibold [&_.t-punct]:text-secondary"><code class="font-[inherit]">{@html rendered}</code></pre>
		<textarea
			bind:this={taRef}
			spellcheck="false"
			{value}
			oninput={onInput}
			onscroll={syncScroll}
			onkeydown={onKeydown}
			onpaste={(e) => recordStructural(e.currentTarget)}
			oncut={(e) => recordStructural(e.currentTarget)}
			onblur={onBlur}
			class="absolute inset-0 m-0 p-[0.7rem] resize-none overflow-auto font-mono text-[0.8rem] leading-[1.5] whitespace-pre border-0 outline-none bg-transparent text-transparent caret-[var(--color-contrast)] [tab-size:2]"
		></textarea>
		{#if suggestions.length}
			<ul
				class="absolute z-30 list-none m-0 p-1 max-h-[12rem] overflow-y-auto bg-bg-super-raised border border-divider rounded-md shadow-floating min-w-[8rem]"
				style="top:{sugTop}px; left:{sugLeft}px"
			>
				{#each suggestions as sug, i (sug)}
					<li>
						<button
							class={`block w-full text-left font-mono text-[0.78rem] px-2 py-[0.2rem] rounded-sm cursor-pointer border-none ${
								i === sugIndex ? 'bg-brand text-on-brand' : 'bg-transparent text-body hover:bg-button-bg'
							}`}
							onmousedown={(e) => {
								e.preventDefault()
								applyCompletion(sug)
							}}
						>
							{sug}
						</button>
					</li>
				{/each}
			</ul>
		{/if}
	</div>
</div>

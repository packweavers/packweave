<script lang="ts">
	import { ChevronDown, ChevronUp, X } from '@lucide/svelte'

	import {
		backspace,
		completionsFor,
		deleteLines,
		duplicateLines,
		formatCode,
		indentLines,
		moveLines,
		newline,
		toggleComment,
		typePair,
		type Edit,
	} from '@/lib/files/editing'
	import { highlight, type Lang } from '@/lib/files/highlight'
	import { lineHunks, lineMarks, revertHunk, type Hunk } from '@/lib/files/linediff'
	import { app } from '@/lib/state/app.svelte'
	import { codeLine } from '@/lib/state/prefs'
	import { plural } from '@/lib/util/format'

	import IconButton from './IconButton.svelte'

	interface Props {
		value: string
		lang: Lang
		base?: string | null
		hints?: ((line: string) => string[]) | null
		readonly?: boolean
		errorLine?: number | null
		revealLine?: number | null
		onchange?: (value: string) => void
	}

	let {
		value = $bindable(),
		lang,
		base = null,
		hints = null,
		readonly = false,
		errorLine = null,
		revealLine = null,
		onchange,
	}: Props = $props()

	interface Snapshot {
		text: string
		start: number
		end: number
	}

	const HISTORY = 500
	const PAD = 12
	const PEEK_LINES = 12

	let area: HTMLTextAreaElement
	let overlay: HTMLPreElement
	let gutter: HTMLDivElement
	let undoStack: Snapshot[] = []
	let redoStack: Snapshot[] = []
	let typingSince = 0
	let typing = false
	let suggestions = $state<string[]>([])
	let suggestionAt = $state(0)
	let suggestionPos = $state({ top: 0, left: 0 })
	let scrollY = $state(0)
	let viewHeight = $state(0)
	let peekAt = $state<number | null>(null)
	let charWidth = 0

	const lineHeight = $derived(codeLine(app.prefs.editorSize))

	$effect(() => {
		void app.prefs.editorSize
		charWidth = 0
	})

	const html = $derived(highlight(value, lang) + '\n')
	const lineCount = $derived(value.split('\n').length)
	const hunks = $derived(base === null ? [] : lineHunks(base, value))
	const marks = $derived(lineMarks(hunks, lineCount))
	const peek = $derived(peekAt === null ? null : (hunks.find((h) => h.start === peekAt) ?? null))
	const peekTop = $derived.by(() => {
		if (!peek) return 0
		const height = 34 + Math.min(peek.oldLines.length, PEEK_LINES) * lineHeight
		const below = PAD + (peek.kind === 'delete' ? peek.start : peek.end) * lineHeight - scrollY
		const above = PAD + peek.start * lineHeight - scrollY - height
		return below + height > viewHeight && above >= 0 ? above : below
	})

	$effect(() => {
		if (revealLine && area) {
			area.scrollTop = Math.max(0, (revealLine - 5) * lineHeight)
			syncScroll()
		}
	})

	function syncScroll(): void {
		overlay.scrollTop = area.scrollTop
		overlay.scrollLeft = area.scrollLeft
		gutter.scrollTop = area.scrollTop
		scrollY = area.scrollTop
	}

	function lineOffset(line: number): number {
		return value
			.split('\n')
			.slice(0, Math.max(0, line))
			.reduce((n, l) => n + l.length + 1, 0)
	}

	function cursorLine(): number {
		return value.slice(0, area.selectionStart).split('\n').length - 1
	}

	function showHunk(hunk: Hunk): void {
		suggestions = []
		peekAt = peekAt === hunk.start ? null : hunk.start
	}

	function goToHunk(hunk: Hunk): void {
		const line = Math.min(hunk.start, lineCount - 1)
		const offset = lineOffset(line)
		area.focus()
		area.setSelectionRange(offset, offset)
		const top = line * lineHeight
		if (top < area.scrollTop || top > area.scrollTop + area.clientHeight - lineHeight * 6) {
			area.scrollTop = Math.max(0, top - area.clientHeight / 3)
		}
		syncScroll()
		suggestions = []
		peekAt = hunk.start
	}

	function stepHunk(direction: 1 | -1): void {
		if (hunks.length === 0) return
		const from = peek?.start ?? cursorLine()
		const ordered = direction === 1 ? hunks : [...hunks].reverse()
		const next = ordered.find((h) => (direction === 1 ? h.start > from : h.start < from))
		goToHunk(next ?? ordered[0])
	}

	function revert(hunk: Hunk): void {
		const next = revertHunk(value, hunk)
		peekAt = null
		apply({ text: next.text, start: next.offset, end: next.offset })
		area.focus()
	}

	function hunkTitle(hunk: Hunk): string {
		if (hunk.kind === 'delete') return `Deleted · ${plural(hunk.oldLines.length, 'line', 'lines')}`
		const lines =
			hunk.end - hunk.start === 1 ? `line ${hunk.start + 1}` : `lines ${hunk.start + 1}–${hunk.end}`
		return `${hunk.kind === 'add' ? 'Added' : 'Changed'} · ${lines}`
	}

	function snapshot(): Snapshot {
		return { text: value, start: area.selectionStart, end: area.selectionEnd }
	}

	function remember(): void {
		undoStack.push(snapshot())
		if (undoStack.length > HISTORY) undoStack.shift()
		redoStack = []
		typing = false
	}

	function rememberTyping(): void {
		const now = performance.now()
		if (!typing || now - typingSince > 400) {
			undoStack.push(snapshot())
			if (undoStack.length > HISTORY) undoStack.shift()
			redoStack = []
			typing = true
		}
		typingSince = now
	}

	function commit(next: Edit | Snapshot): void {
		area.value = next.text
		area.setSelectionRange(next.start, next.end)
		value = next.text
		onchange?.(next.text)
		suggestions = []
		syncScroll()
	}

	function apply(edit: Edit | null): boolean {
		if (!edit) return false
		remember()
		commit(edit)
		return true
	}

	function undo(): void {
		const previous = undoStack.pop()
		if (!previous) return
		redoStack.push(snapshot())
		typing = false
		commit(previous)
	}

	function redo(): void {
		const next = redoStack.pop()
		if (!next) return
		undoStack.push(snapshot())
		typing = false
		commit(next)
	}

	export function format(): void {
		if (lang === 'text') return
		apply({ text: formatCode(value, lang), start: 0, end: 0 })
	}

	export function focusLine(line: number): void {
		const lines = value.split('\n')
		const offset = lines.slice(0, Math.max(0, line - 1)).reduce((n, l) => n + l.length + 1, 0)
		area.focus()
		area.setSelectionRange(offset, offset)
		area.scrollTop = Math.max(0, (line - 5) * lineHeight)
		syncScroll()
	}

	function measure(): void {
		const probe = document.createElement('span')
		probe.style.font = getComputedStyle(area).font
		probe.style.position = 'absolute'
		probe.style.visibility = 'hidden'
		probe.style.whiteSpace = 'pre'
		probe.textContent = 'MMMMMMMMMM'
		document.body.appendChild(probe)
		charWidth = probe.getBoundingClientRect().width / 10
		probe.remove()
	}

	function wordBeforeCursor(): { word: string; start: number } {
		const at = area.selectionStart
		if (hints) {
			const start = value.lastIndexOf('\n', at - 1) + 1
			return { word: value.slice(start, at), start }
		}
		const match = /[A-Za-z_$][\w$-]*$/.exec(value.slice(0, at))
		return { word: match?.[0] ?? '', start: match ? at - match[0].length : at }
	}

	function suggest(): void {
		if ((lang === 'text' && !hints) || area.selectionStart !== area.selectionEnd) {
			suggestions = []
			return
		}
		const { word, start } = wordBeforeCursor()
		if (hints) {
			suggestions = word.trim() && !word.trimStart().startsWith('#') ? hints(word) : []
		} else {
			suggestions = word.length >= 2 ? completionsFor(value, lang, word) : []
		}
		suggestionAt = 0
		if (suggestions.length === 0) return
		if (!charWidth) measure()
		const lines = value.slice(0, start).split('\n')
		suggestionPos = {
			top: PAD + lines.length * lineHeight - area.scrollTop,
			left: PAD + lines[lines.length - 1].length * charWidth - area.scrollLeft,
		}
	}

	function complete(word: string): void {
		const { start } = wordBeforeCursor()
		const at = area.selectionStart
		apply({
			text: value.slice(0, start) + word + value.slice(at),
			start: start + word.length,
			end: start + word.length,
		})
		if (hints && word.endsWith('/')) suggest()
	}

	function oninput(): void {
		value = area.value
		onchange?.(area.value)
		suggest()
	}

	function onkeydown(event: KeyboardEvent): void {
		if (event.altKey && event.key === 'F5') {
			event.preventDefault()
			stepHunk(event.shiftKey ? -1 : 1)
			return
		}
		if (event.key === 'Escape' && peek && suggestions.length === 0) {
			event.preventDefault()
			peekAt = null
			return
		}
		if (readonly) return
		const mod = event.metaKey || event.ctrlKey
		const key = event.key.toLowerCase()
		const start = area.selectionStart
		const end = area.selectionEnd
		const handled = (done: boolean) => {
			if (done) event.preventDefault()
			return done
		}

		if (mod && !event.altKey && key === 'z') {
			event.preventDefault()
			if (event.shiftKey) {
				redo()
			} else {
				undo()
			}
			return
		}
		if (mod && !event.altKey && key === 'y') {
			event.preventDefault()
			redo()
			return
		}
		if (mod && key === '/') return void handled(apply(toggleComment(value, start, end, lang)))
		if (event.altKey && !mod && (event.key === 'ArrowUp' || event.key === 'ArrowDown')) {
			return void handled(
				apply(moveLines(value, start, end, event.key === 'ArrowUp' ? -1 : 1)) || true,
			)
		}
		if (mod && event.shiftKey && key === 'd') {
			return void handled(apply(duplicateLines(value, start, end)))
		}
		if (mod && event.shiftKey && key === 'k') {
			return void handled(apply(deleteLines(value, start, end)))
		}

		if (suggestions.length > 0) {
			if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
				event.preventDefault()
				const step = event.key === 'ArrowDown' ? 1 : -1
				suggestionAt = (suggestionAt + step + suggestions.length) % suggestions.length
				return
			}
			if (event.key === 'Enter' || event.key === 'Tab') {
				event.preventDefault()
				complete(suggestions[suggestionAt])
				return
			}
			if (event.key === 'Escape') {
				event.preventDefault()
				suggestions = []
				return
			}
		}

		if (
			event.key.startsWith('Arrow') ||
			['Home', 'End', 'PageUp', 'PageDown'].includes(event.key)
		) {
			typing = false
			return
		}
		if (!mod && !event.altKey && event.key.length === 1) {
			if (handled(apply(typePair(value, start, end, event.key)))) return
		}
		if (event.key === 'Backspace' && !mod && handled(apply(backspace(value, start, end)))) return
		if (event.key === 'Enter' && !mod) return void handled(apply(newline(value, start, end)))
		if (event.key === 'Tab' && !mod) {
			return void handled(apply(indentLines(value, start, end, event.shiftKey)))
		}

		if (
			(event.key.length === 1 && !mod && !event.altKey) ||
			event.key === 'Backspace' ||
			event.key === 'Delete'
		) {
			rememberTyping()
		}
	}
</script>

<div class="editor">
	<div bind:this={gutter} class="gutter code" aria-hidden="true">
		{#each { length: lineCount } as _, i (i)}
			{@const mark = marks.lines[i]}
			{@const gone = marks.deleted[i]}
			<div class="line" class:error={errorLine === i + 1} class:reveal={revealLine === i + 1}>
				{i + 1}
				{#if mark}
					<button
						class="git-mark {mark.kind}"
						class:open={peek === mark}
						tabindex="-1"
						aria-label={hunkTitle(mark)}
						onclick={() => showHunk(mark)}
					></button>
				{/if}
				{#if gone}
					<button
						class="git-mark delete"
						class:after={gone.start > i}
						class:open={peek === gone}
						tabindex="-1"
						aria-label={hunkTitle(gone)}
						onclick={() => showHunk(gone)}
					></button>
				{/if}
			</div>
		{/each}
	</div>
	<div class="surface" bind:clientHeight={viewHeight}>
		<pre bind:this={overlay} class="overlay code" aria-hidden="true"><code>{@html html}</code></pre>
		<textarea
			bind:this={area}
			class="input code"
			spellcheck="false"
			{readonly}
			{value}
			{oninput}
			{onkeydown}
			onscroll={syncScroll}
			onpaste={remember}
			oncut={remember}
			onmousedown={() => (peekAt = null)}
			onblur={() => (suggestions = [])}></textarea>
		{#if hunks.length > 0}
			<div class="overview" aria-hidden="true">
				{#each hunks as hunk (hunk.start)}
					<span
						class={hunk.kind}
						style:top="{(hunk.start / lineCount) * 100}%"
						style:height="max(2px, {((hunk.end - hunk.start) / lineCount) * 100}%)"
					></span>
				{/each}
			</div>
		{/if}
		{#if peek}
			<div class="peek" style:top="{peekTop}px">
				<div class="peek-bar">
					<span class="small">{hunkTitle(peek)}</span>
					<span class="spacer"></span>
					{#if hunks.length > 1}
						<IconButton
							icon={ChevronUp}
							label="Previous change"
							size={14}
							onclick={() => stepHunk(-1)}
						/>
						<IconButton
							icon={ChevronDown}
							label="Next change"
							size={14}
							onclick={() => stepHunk(1)}
						/>
					{/if}
					{#if !readonly}
						<button class="btn" onclick={() => revert(peek)}>Revert</button>
					{/if}
					<IconButton icon={X} label="Close" size={14} onclick={() => (peekAt = null)} />
				</div>
				{#if peek.oldLines.length > 0}
					<pre class="peek-old code"><code>{@html highlight(peek.oldLines.join('\n'), lang)}</code
						></pre>
				{/if}
			</div>
		{/if}
		{#if suggestions.length > 0}
			<div
				class="suggestions menu"
				style:top="{suggestionPos.top}px"
				style:left="{suggestionPos.left}px"
			>
				{#each suggestions as word, i (word)}
					<button
						class="menu-item mono"
						class:active={i === suggestionAt}
						onmousedown={(e) => {
							e.preventDefault()
							complete(word)
						}}
					>
						{word}
					</button>
				{/each}
			</div>
		{/if}
	</div>
</div>

<style>
	.editor {
		position: relative;
		flex: 1;
		min-height: 0;
		display: flex;
		overflow: hidden;
		background: var(--doc);
	}

	.gutter {
		flex: none;
		overflow: hidden;
		padding: 12px 16px 12px 14px;
		min-width: 56px;
		text-align: right;
		user-select: none;
		-webkit-user-select: none;
	}

	.gutter .line {
		position: relative;
	}

	.gutter .error {
		color: var(--red);
		font-weight: 600;
	}

	.git-mark {
		position: absolute;
		top: 0;
		bottom: 0;
		right: -16px;
		width: 15px;
		padding: 0;
		border: 0;
		background: none;
		cursor: pointer;
	}

	.git-mark::before {
		content: '';
		position: absolute;
		top: 0;
		bottom: 0;
		left: 6px;
		width: 3px;
		background: currentColor;
	}

	.git-mark.add {
		color: var(--green);
	}

	.git-mark.change {
		color: var(--blue);
	}

	.git-mark.delete {
		top: -5px;
		bottom: auto;
		height: 10px;
		color: var(--red);
	}

	.git-mark.delete.after {
		top: auto;
		bottom: -5px;
	}

	.git-mark.delete::before {
		top: 1px;
		bottom: auto;
		left: 5px;
		width: 0;
		height: 0;
		border-top: 4px solid transparent;
		border-bottom: 4px solid transparent;
		border-left: 6px solid currentColor;
		background: none;
	}

	.git-mark.open::before,
	.git-mark:hover::before {
		left: 5px;
		width: 5px;
	}

	.git-mark.delete.open::before,
	.git-mark.delete:hover::before {
		left: 5px;
		width: 0;
		border-left-width: 8px;
	}

	.gutter .reveal {
		background: var(--hit-bg);
	}

	.surface {
		position: relative;
		flex: 1;
		min-width: 0;
		overflow: hidden;
	}

	.overlay,
	.input {
		position: absolute;
		inset: 0;
		margin: 0;
		padding: 12px;
		border: 0;
		overflow: hidden;
	}

	.overlay {
		pointer-events: none;
	}

	.overlay code {
		font: inherit;
	}

	.input {
		overflow: auto;
		resize: none;
		background: transparent;
		color: transparent;
		caret-color: var(--text);
		outline: none;
		user-select: text;
		-webkit-user-select: text;
	}

	.input::selection {
		background: var(--blue-soft);
		color: transparent;
	}

	.suggestions {
		position: absolute;
		min-width: 140px;
		max-height: 200px;
	}

	.overview {
		position: absolute;
		top: 0;
		right: 0;
		bottom: 0;
		width: 6px;
		pointer-events: none;
	}

	.overview > span {
		position: absolute;
		left: 0;
		right: 0;
		min-height: 2px;
		opacity: 0.8;
	}

	.overview > .add {
		background: var(--green);
	}

	.overview > .change {
		background: var(--blue);
	}

	.overview > .delete {
		height: 2px;
		background: var(--red);
	}

	.peek {
		position: absolute;
		left: 0;
		right: 12px;
		max-width: 720px;
		z-index: 5;
		display: flex;
		flex-direction: column;
		background: var(--overlay);
		border: 1px solid var(--edge);
	}

	.peek-bar {
		display: flex;
		align-items: center;
		gap: 4px;
		height: 32px;
		padding: 0 4px 0 12px;
		border-bottom: 1px solid var(--line);
	}

	.peek-bar .btn {
		height: 24px;
	}

	.peek-old {
		margin: 0;
		padding: 0 12px;
		max-height: 240px;
		overflow: auto;
		background: var(--del-bg);
	}

	.peek-old code {
		font: inherit;
	}
</style>

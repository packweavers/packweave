export type HunkKind = 'add' | 'change' | 'delete'

export interface Hunk {
	kind: HunkKind
	start: number
	end: number
	oldLines: string[]
}

export interface LineMarks {
	lines: (Hunk | null)[]
	deleted: (Hunk | null)[]
}

const LIMIT = 2000

type Op = 'same' | 'del' | 'ins'

function splitLines(text: string): string[] {
	return text.split('\n').map((line) => (line.endsWith('\r') ? line.slice(0, -1) : line))
}

function shortestEdit(a: string[], b: string[]): Op[] | null {
	const n = a.length
	const m = b.length
	const offset = n + m + 1
	const v = new Int32Array(2 * offset + 1)
	const trace: Int32Array[] = []
	let found = false
	for (let d = 0; d <= n + m && !found; d++) {
		if (d > LIMIT) return null
		trace.push(v.slice(offset - d - 1, offset + d + 2))
		for (let k = -d; k <= d; k += 2) {
			const down = k === -d || (k !== d && v[offset + k - 1] < v[offset + k + 1])
			let x = down ? v[offset + k + 1] : v[offset + k - 1] + 1
			let y = x - k
			while (x < n && y < m && a[x] === b[y]) {
				x++
				y++
			}
			v[offset + k] = x
			if (x >= n && y >= m) {
				found = true
				break
			}
		}
	}
	const ops: Op[] = []
	let x = n
	let y = m
	for (let d = trace.length - 1; d > 0; d--) {
		const prev = trace[d]
		const at = (k: number) => prev[k + d + 1]
		const k = x - y
		const down = k === -d || (k !== d && at(k - 1) < at(k + 1))
		const prevK = down ? k + 1 : k - 1
		const prevX = at(prevK)
		const prevY = prevX - prevK
		const startX = down ? prevX : prevX + 1
		while (x > startX) {
			ops.push('same')
			x--
			y--
		}
		ops.push(down ? 'ins' : 'del')
		x = prevX
		y = prevY
	}
	while (x > 0) {
		ops.push('same')
		x--
	}
	return ops.reverse()
}

function classify(start: number, end: number, oldLines: string[]): Hunk {
	const kind: HunkKind = oldLines.length === 0 ? 'add' : end === start ? 'delete' : 'change'
	return { kind, start, end, oldLines }
}

export function lineHunks(base: string, text: string): Hunk[] {
	const a = splitLines(base)
	const b = splitLines(text)
	let head = 0
	while (head < a.length && head < b.length && a[head] === b[head]) head++
	let tail = 0
	while (
		tail < a.length - head &&
		tail < b.length - head &&
		a[a.length - 1 - tail] === b[b.length - 1 - tail]
	) {
		tail++
	}
	const midA = a.slice(head, a.length - tail)
	const midB = b.slice(head, b.length - tail)
	if (midA.length === 0 && midB.length === 0) return []
	const ops = shortestEdit(midA, midB)
	if (!ops) return [classify(head, head + midB.length, midA)]
	const hunks: Hunk[] = []
	let ai = 0
	let bi = 0
	let open: { a: number; b: number } | null = null
	const close = () => {
		if (!open) return
		hunks.push(classify(head + open.b, head + bi, midA.slice(open.a, ai)))
		open = null
	}
	for (const op of ops) {
		if (op === 'same') {
			close()
			ai++
			bi++
			continue
		}
		open ??= { a: ai, b: bi }
		if (op === 'del') {
			ai++
		} else {
			bi++
		}
	}
	close()
	return hunks
}

export function lineMarks(hunks: Hunk[], lineCount: number): LineMarks {
	const lines: (Hunk | null)[] = new Array(lineCount).fill(null)
	const deleted: (Hunk | null)[] = new Array(lineCount).fill(null)
	for (const hunk of hunks) {
		if (hunk.kind === 'delete') {
			deleted[Math.min(hunk.start, lineCount - 1)] = hunk
			continue
		}
		for (let line = hunk.start; line < hunk.end && line < lineCount; line++) lines[line] = hunk
	}
	return { lines, deleted }
}

export function revertHunk(text: string, hunk: Hunk): { text: string; offset: number } {
	const eol = text.includes('\r\n') ? '\r\n' : '\n'
	const lines = text.split(eol)
	lines.splice(hunk.start, hunk.end - hunk.start, ...hunk.oldLines)
	const offset = lines.slice(0, hunk.start).reduce((n, line) => n + line.length + eol.length, 0)
	return { text: lines.join(eol), offset }
}

export type DiffLineKind = 'context' | 'add' | 'del' | 'gap'

export interface DiffLine {
	kind: DiffLineKind
	oldNo: number | null
	newNo: number | null
	text: string
}

export interface DiffRow {
	left: DiffLine | null
	right: DiffLine | null
}

export function parseDiff(raw: string): DiffLine[] {
	const lines: DiffLine[] = []
	let oldNo = 0
	let newNo = 0
	let inHunk = false
	for (const line of raw.replace(/\r\n/g, '\n').split('\n')) {
		const hunk = /^@@ -(\d+)(?:,\d+)? \+(\d+)(?:,\d+)? @@/.exec(line)
		if (hunk) {
			if (inHunk) lines.push({ kind: 'gap', oldNo: null, newNo: null, text: '⋯' })
			oldNo = Number(hunk[1])
			newNo = Number(hunk[2])
			inHunk = true
			continue
		}
		if (!inHunk) {
			if (line.startsWith('+') && !line.startsWith('+++')) {
				lines.push({ kind: 'add', oldNo: null, newNo: ++newNo, text: line.slice(1) })
			}
			continue
		}
		if (line.startsWith('\\')) continue
		if (line.startsWith('+')) {
			lines.push({ kind: 'add', oldNo: null, newNo: newNo++, text: line.slice(1) })
		} else if (line.startsWith('-')) {
			lines.push({ kind: 'del', oldNo: oldNo++, newNo: null, text: line.slice(1) })
		} else if (line.startsWith(' ') || line === '') {
			lines.push({ kind: 'context', oldNo: oldNo++, newNo: newNo++, text: line.slice(1) })
		}
	}
	while (
		lines.length > 0 &&
		lines[lines.length - 1].kind === 'context' &&
		lines[lines.length - 1].text === ''
	) {
		lines.pop()
	}
	return lines
}

export function sideBySide(lines: DiffLine[]): DiffRow[] {
	const rows: DiffRow[] = []
	let i = 0
	while (i < lines.length) {
		const line = lines[i]
		if (line.kind === 'context' || line.kind === 'gap') {
			rows.push({ left: line, right: line })
			i++
			continue
		}
		const dels: DiffLine[] = []
		const adds: DiffLine[] = []
		while (i < lines.length && lines[i].kind === 'del') dels.push(lines[i++])
		while (i < lines.length && lines[i].kind === 'add') adds.push(lines[i++])
		for (let k = 0; k < Math.max(dels.length, adds.length); k++) {
			rows.push({ left: dels[k] ?? null, right: adds[k] ?? null })
		}
	}
	return rows
}

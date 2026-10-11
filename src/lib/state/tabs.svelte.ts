import { tasks } from './tasks.svelte'

export type DocKind =
	| 'content'
	| 'mod'
	| 'add'
	| 'file'
	| 'diff'
	| 'changes'
	| 'history'
	| 'branches'
	| 'compare'
	| 'conflicts'
	| 'changelog'
	| 'instance'
	| 'release'
	| 'pack'
	| 'bisect'
	| 'problems'
	| 'output'
	| 'gamelog'

const KINDS: DocKind[] = [
	'content',
	'mod',
	'add',
	'file',
	'diff',
	'changes',
	'history',
	'branches',
	'compare',
	'conflicts',
	'changelog',
	'instance',
	'release',
	'pack',
	'bisect',
	'problems',
	'output',
	'gamelog',
]

export interface Doc {
	id: string
	kind: DocKind
	arg: string
	preview: boolean
	pinned: boolean
}

export interface Group {
	docs: Doc[]
	active: string | null
}

export interface OpenOptions {
	preview?: boolean
	side?: boolean
}

const SINGLE_KINDS: DocKind[] = [
	'add',
	'history',
	'pack',
	'bisect',
	'problems',
	'output',
	'gamelog',
]

export function isDocKind(kind: string): kind is DocKind {
	return (KINDS as string[]).includes(kind)
}

export function docId(kind: DocKind, arg = ''): string {
	return arg && !SINGLE_KINDS.includes(kind) ? `${kind}:${arg}` : kind
}

export function bareDoc(kind: DocKind): Doc {
	return { id: docId(kind), kind, arg: '', preview: false, pinned: false }
}

const CLOSED_LIMIT = 20

class Tabs {
	groups = $state<Group[]>([{ docs: [], active: null }])
	focused = $state(0)
	#closed: Doc[] = []
	#key: string | null = null

	get group(): Group {
		return this.groups[this.focused] ?? this.groups[0]
	}

	get active(): Doc | null {
		const group = this.group
		return group.docs.find((d) => d.id === group.active) ?? null
	}

	get all(): Doc[] {
		return this.groups.flatMap((g) => g.docs)
	}

	find(id: string): { group: number; doc: Doc } | null {
		for (const [index, group] of this.groups.entries()) {
			const doc = group.docs.find((d) => d.id === id)
			if (doc) return { group: index, doc }
		}
		return null
	}

	isOpen(kind: DocKind, arg = ''): boolean {
		return this.find(docId(kind, arg)) !== null
	}

	open(kind: DocKind, arg = '', options: OpenOptions = {}): void {
		const id = docId(kind, arg)
		const existing = this.find(id)
		if (existing && !options.side) {
			this.focused = existing.group
			this.groups[existing.group].active = id
			if (!options.preview) existing.doc.preview = false
			if (arg) existing.doc.arg = arg
			this.persist()
			return
		}
		if (existing) this.#remove(existing.group, id)
		const target = options.side ? this.#otherGroup() : this.focused
		const group = this.groups[target]
		const doc: Doc = { id, kind, arg, preview: options.preview === true, pinned: false }
		const previewAt = options.preview ? group.docs.findIndex((d) => d.preview) : -1
		if (previewAt >= 0) group.docs[previewAt] = doc
		else {
			const after = group.docs.findIndex((d) => d.id === group.active)
			group.docs.splice(after + 1, 0, doc)
		}
		group.active = id
		this.focused = target
		this.persist()
	}

	setArg(id: string, arg: string): void {
		const found = this.find(id)
		if (!found || found.doc.arg === arg) return
		found.doc.arg = arg
		this.persist()
	}

	rekey(id: string, arg: string): void {
		const found = this.find(id)
		if (!found) return
		const next = docId(found.doc.kind, arg)
		if (next === id) return
		const group = this.groups[found.group]
		const existing = this.find(next)
		if (existing) {
			if (existing.group === found.group && group.active === id) group.active = next
			this.#remove(found.group, id)
		} else {
			found.doc.id = next
			found.doc.arg = arg
			if (group.active === id) group.active = next
		}
		this.persist()
	}

	keep(id: string): void {
		const found = this.find(id)
		if (found) found.doc.preview = false
		this.persist()
	}

	activate(id: string): void {
		const found = this.find(id)
		if (!found) return
		this.focused = found.group
		this.groups[found.group].active = id
		this.persist()
	}

	close(id: string): void {
		const found = this.find(id)
		if (!found || found.doc.pinned) return
		this.#closed = [...this.#closed.slice(-(CLOSED_LIMIT - 1)), { ...found.doc, preview: false }]
		this.#remove(found.group, id)
		this.persist()
	}

	closeWhere(test: (doc: Doc) => boolean): void {
		for (const doc of this.all.filter(test)) this.close(doc.id)
	}

	reopen(): void {
		const doc = this.#closed.pop()
		if (doc) this.open(doc.kind, doc.arg)
	}

	pin(id: string, pinned: boolean): void {
		const found = this.find(id)
		if (!found) return
		const group = this.groups[found.group]
		found.doc.pinned = pinned
		found.doc.preview = false
		group.docs = [...group.docs.filter((d) => d.pinned), ...group.docs.filter((d) => !d.pinned)]
		this.persist()
	}

	cycle(step: 1 | -1): void {
		const group = this.group
		if (group.docs.length < 2) return
		const index = group.docs.findIndex((d) => d.id === group.active)
		const next = (index + step + group.docs.length) % group.docs.length
		group.active = group.docs[next].id
		this.persist()
	}

	goto(position: number): void {
		const doc = this.group.docs[position - 1]
		if (doc) this.activate(doc.id)
	}

	split(): void {
		const doc = this.active
		if (!doc) return
		this.open(doc.kind, doc.arg, { side: true })
	}

	moveTo(id: string, group: number, index: number): void {
		const found = this.find(id)
		if (!found) return
		if (group >= this.groups.length) this.groups.push({ docs: [], active: null })
		const source = this.groups[found.group]
		const from = source.docs.findIndex((d) => d.id === id)
		const at = found.group === group && index > from ? index - 1 : index
		source.docs = source.docs.filter((d) => d.id !== id)
		const target = this.groups[group]
		target.docs.splice(Math.min(at, target.docs.length), 0, found.doc)
		target.active = id
		if (source.active === id) source.active = source.docs[0]?.id ?? null
		this.focused = group
		this.#dropEmptyGroups()
		this.persist()
	}

	detach(id: string): void {
		const found = this.find(id)
		if (!found) return
		this.#remove(found.group, id)
		this.persist()
	}

	reset(): void {
		this.groups = [{ docs: [], active: null }]
		this.focused = 0
		this.#closed = []
	}

	restore(dir: string, remember = true): void {
		this.#key = remember ? `tabs:${dir}` : null
		this.reset()
		if (!this.#key) return
		try {
			const saved = JSON.parse(localStorage.getItem(this.#key) ?? 'null') as {
				groups?: Group[]
				focused?: number
			} | null
			const seen = new Set<string>()
			const groups = (saved?.groups ?? [])
				.map((g) => {
					const docs: Doc[] = []
					let active: string | null = null
					for (const d of g.docs ?? []) {
						if (!KINDS.includes(d.kind)) continue
						const id = docId(d.kind, d.arg)
						if (d.id === g.active) active = id
						if (seen.has(id)) continue
						seen.add(id)
						docs.push({ ...d, id })
					}
					const shown = docs.some((d) => d.id === active) ? active : (docs[0]?.id ?? null)
					return { docs, active: shown }
				})
				.filter((g) => g.docs.length > 0)
			if (groups.length > 0) {
				this.groups = groups.slice(0, 2)
				this.focused = Math.min(saved?.focused ?? 0, this.groups.length - 1)
			}
		} catch (error) {
			tasks.fail('Restoring the open tabs', error)
			this.reset()
		}
	}

	persist(): void {
		if (!this.#key) return
		try {
			localStorage.setItem(
				this.#key,
				JSON.stringify({ groups: $state.snapshot(this.groups), focused: this.focused }),
			)
		} catch (error) {
			tasks.fail('Saving the open tabs', error)
		}
	}

	#otherGroup(): number {
		if (this.groups.length < 2) this.groups.push({ docs: [], active: null })
		return this.focused === 0 ? 1 : 0
	}

	#remove(groupIndex: number, id: string): void {
		const group = this.groups[groupIndex]
		const index = group.docs.findIndex((d) => d.id === id)
		if (index < 0) return
		group.docs.splice(index, 1)
		if (group.active === id) {
			group.active = group.docs[Math.min(index, group.docs.length - 1)]?.id ?? null
		}
		this.#dropEmptyGroups()
	}

	#dropEmptyGroups(): void {
		if (this.groups.length < 2) return
		const kept = this.groups.filter((g) => g.docs.length > 0)
		this.groups = kept.length ? kept : [{ docs: [], active: null }]
		this.focused = Math.min(this.focused, this.groups.length - 1)
	}
}

export const tabs = new Tabs()

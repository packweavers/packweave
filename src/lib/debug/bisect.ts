export interface BisectMod {
	file: string
	name: string
	key: string | null
	needs: string[]
}

export interface BisectStep {
	suspects: string[]
	testing: string[]
	sawProblem: boolean
}

export interface BisectSession {
	instanceDir: string
	mods: BisectMod[]
	original: string[]
	keep: string[]
	suspects: string[]
	testing: string[]
	history: BisectStep[]
	found: string[] | null
	checking: boolean
	checked: boolean | null
	sawProblem: boolean
}

function byFile(mods: BisectMod[]): Map<string, BisectMod> {
	return new Map(mods.map((m) => [m.file, m]))
}

function closure(files: Iterable<string>, mods: BisectMod[]): Set<string> {
	const map = byFile(mods)
	const on = new Set(files)
	const queue = [...on]
	while (queue.length > 0) {
		const file = queue.pop()!
		for (const dep of map.get(file)?.needs ?? []) {
			if (!on.has(dep)) {
				on.add(dep)
				queue.push(dep)
			}
		}
	}
	return on
}

function dependents(file: string, mods: BisectMod[]): Set<string> {
	const out = new Set<string>([file])
	let grew = true
	while (grew) {
		grew = false
		for (const mod of mods) {
			if (!out.has(mod.file) && mod.needs.some((n) => out.has(n))) {
				out.add(mod.file)
				grew = true
			}
		}
	}
	return out
}

export function enabledFor(session: BisectSession): Set<string> {
	if (session.checking && session.found) {
		const off = new Set<string>()
		for (const file of session.found) {
			for (const dependent of dependents(file, session.mods)) off.add(dependent)
		}
		for (const file of session.keep) off.delete(file)
		return closure(
			session.original.filter((f) => !off.has(f)),
			session.mods,
		)
	}
	const suspects = new Set(session.suspects)
	const base = session.original.filter((f) => !suspects.has(f))
	return closure([...base, ...session.testing, ...session.keep], session.mods)
}

function split(session: BisectSession, suspects: string[]): string[] {
	const half = Math.ceil(suspects.length / 2)
	const options = [suspects.slice(0, half), suspects.slice(half)]
	for (const testing of options) {
		const on = enabledFor({ ...session, suspects, testing, checking: false })
		if (suspects.some((f) => !on.has(f)) && suspects.some((f) => on.has(f))) return testing
	}
	return []
}

export function begin(
	instanceDir: string,
	mods: BisectMod[],
	original: string[],
	suspects: string[],
	keep: string[],
): BisectSession {
	const session: BisectSession = {
		instanceDir,
		mods,
		original,
		keep,
		suspects,
		testing: [],
		history: [],
		found: null,
		checking: false,
		checked: null,
		sawProblem: false,
	}
	return advance(session, suspects)
}

function advance(session: BisectSession, suspects: string[]): BisectSession {
	if (suspects.length <= 1) return { ...session, suspects, testing: [], found: suspects }
	const testing = split(session, suspects)
	if (testing.length === 0) return { ...session, suspects, testing: [], found: suspects }
	return { ...session, suspects, testing, found: null }
}

export function answer(session: BisectSession, happened: boolean): BisectSession {
	if (session.checking) return { ...session, checking: false, checked: !happened }
	const on = enabledFor(session)
	const next = session.suspects.filter((f) => on.has(f) === happened)
	const history = [
		...session.history,
		{ suspects: session.suspects, testing: session.testing, sawProblem: session.sawProblem },
	]
	return advance(
		{ ...session, history, checked: null, sawProblem: session.sawProblem || happened },
		next,
	)
}

export function back(session: BisectSession): BisectSession {
	if (session.checking) return { ...session, checking: false }
	const last = session.history[session.history.length - 1]
	if (!last) return session
	return {
		...session,
		history: session.history.slice(0, -1),
		suspects: last.suspects,
		testing: last.testing,
		sawProblem: last.sawProblem,
		found: null,
		checked: null,
	}
}

export function stepsLeft(session: BisectSession): number {
	return session.found ? 0 : Math.ceil(Math.log2(Math.max(session.suspects.length, 1)))
}

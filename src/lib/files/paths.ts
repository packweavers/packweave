export function withFolders(files: string[]): string[] {
	const folders = new Set<string>()
	for (const file of files) {
		for (let i = file.indexOf('/'); i !== -1; i = file.indexOf('/', i + 1)) {
			folders.add(file.slice(0, i + 1))
		}
	}
	return [...folders, ...files]
}

function rank(path: string, query: string): number {
	if (path.startsWith(query)) {
		const rest = path.slice(query.length)
		const slash = rest.indexOf('/')
		return slash === -1 || slash === rest.length - 1 ? 0 : 1
	}
	if (path.includes(`/${query}`)) return 2
	return path.includes(query) ? 3 : -1
}

export function pathHints(paths: string[], typed: string, limit = 8): string[] {
	const text = typed.trim().replace(/\\/g, '/')
	const anchor = /^!?\/?/.exec(text)?.[0] ?? ''
	const query = text.slice(anchor.length).replace(/^\/+/, '').toLowerCase()
	if (!query) return []
	const ranked: { path: string; score: number }[] = []
	for (const path of paths) {
		const lower = path.toLowerCase()
		if (lower === query) continue
		const score = rank(lower, query)
		if (score >= 0) ranked.push({ path, score })
	}
	ranked.sort(
		(a, b) => a.score - b.score || a.path.length - b.path.length || a.path.localeCompare(b.path),
	)
	return ranked.slice(0, limit).map((r) => anchor + r.path)
}

export function keyHints(keys: string[], typed: string, limit = 8): string[] {
	const query = typed.trim().toLowerCase()
	const starts = keys.filter((k) => k.toLowerCase().startsWith(query) && k.toLowerCase() !== query)
	const contains = keys.filter(
		(k) => !k.toLowerCase().startsWith(query) && k.toLowerCase().includes(query),
	)
	return [...starts, ...contains].slice(0, limit)
}

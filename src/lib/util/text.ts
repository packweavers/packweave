export function fuzzyScore(query: string, text: string): number | null {
	const q = query.toLowerCase()
	const t = text.toLowerCase()
	if (!q) return 0
	const direct = t.indexOf(q)
	if (direct >= 0) return 1000 - direct * 2 - (t.length - q.length) * 0.1
	let score = 0
	let at = 0
	let streak = 0
	for (const ch of q) {
		const found = t.indexOf(ch, at)
		if (found < 0) return null
		streak = found === at ? streak + 1 : 0
		score += 10 + streak * 5 - (found - at)
		at = found + 1
	}
	return score
}

export function slugify(text: string): string {
	return (
		text
			.trim()
			.replace(/[^a-z0-9-_ ]/gi, '')
			.replace(/\s+/g, '-') || 'modpack'
	)
}

export function githubRepo(input: string): string | null {
	const path = input
		.trim()
		.replace(/^git@github\.com:/i, '')
		.replace(/^(https?:\/\/)?(www\.)?github\.com\//i, '')
	const [owner = '', raw = ''] = path.split('/')
	const name = raw.replace(/\.git$/i, '')
	if (!/^[A-Za-z0-9-]+$/.test(owner) || !/^[A-Za-z0-9._-]+$/.test(name)) return null
	if (name === '.' || name === '..') return null
	return `${owner}/${name}`
}

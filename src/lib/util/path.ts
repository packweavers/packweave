function normalize(path: string): string {
	return path.replace(/\\/g, '/')
}

export function basename(path: string): string {
	const parts = normalize(path).replace(/\/+$/, '').split('/')
	return parts[parts.length - 1] ?? path
}

export function instanceFolderName(gameDir: string): string {
	return basename(normalize(gameDir).replace(/\/\.?minecraft\/?$/, ''))
}

export function dirname(path: string): string {
	const clean = normalize(path).replace(/\/+$/, '')
	const i = clean.lastIndexOf('/')
	return i <= 0 ? '' : clean.slice(0, i)
}

export function join(...parts: string[]): string {
	return parts
		.filter((p) => p.length > 0)
		.map((p, i) =>
			i === 0 ? normalize(p).replace(/\/+$/, '') : normalize(p).replace(/^\/+|\/+$/g, ''),
		)
		.join('/')
}

export function extension(path: string): string {
	const name = basename(path)
	const i = name.lastIndexOf('.')
	return i <= 0 ? '' : name.slice(i + 1).toLowerCase()
}

export function shortenHome(path: string): string {
	const clean = normalize(path)
	const home = /^(\/Users\/[^/]+|\/home\/[^/]+|[A-Za-z]:\/Users\/[^/]+)/.exec(clean)
	return home ? `~${clean.slice(home[1].length)}` : clean
}

export function sameDir(a: string, b: string): boolean {
	const clean = (p: string) => normalize(p).replace(/\/+$/, '').toLowerCase()
	return clean(a) === clean(b)
}

export function globMatches(pattern: string, path: string): boolean {
	let glob = pattern.replace(/\\/g, '/').replace(/^\/+/, '')
	if (glob.endsWith('/')) glob += '**'
	let source = ''
	for (let i = 0; i < glob.length; i++) {
		const c = glob[i]
		if (c === '*' && glob[i + 1] === '*') {
			source += '.*'
			i++
		} else if (c === '*') {
			source += '[^/]*'
		} else if (c === '?') {
			source += '[^/]'
		} else {
			source += c.replace(/[.+^${}()|[\]\\-]/g, '\\$&')
		}
	}
	return new RegExp(`^${source}$`).test(path)
}

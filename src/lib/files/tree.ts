export interface TreeNode<T> {
	name: string
	path: string
	children: TreeNode<T>[]
	item: T | null
	count: number
}

export function buildTree<T>(items: T[], pathOf: (item: T) => string): TreeNode<T>[] {
	const root: TreeNode<T> = { name: '', path: '', children: [], item: null, count: 0 }
	for (const item of items) {
		const parts = pathOf(item).split('/')
		let node = root
		for (const [i, part] of parts.slice(0, -1).entries()) {
			let folder = node.children.find((c) => c.item === null && c.name === part)
			if (!folder) {
				folder = {
					name: part,
					path: parts.slice(0, i + 1).join('/'),
					children: [],
					item: null,
					count: 0,
				}
				node.children.push(folder)
			}
			node = folder
		}
		node.children.push({
			name: parts[parts.length - 1],
			path: parts.join('/'),
			children: [],
			item,
			count: 1,
		})
	}
	return finish(root, true).children
}

function finish<T>(node: TreeNode<T>, root: boolean): TreeNode<T> {
	if (node.item !== null) return node
	const children = node.children.map((child) => finish(child, false))
	if (!root && children.length === 1) {
		return { ...children[0], name: `${node.name}/${children[0].name}` }
	}
	children.sort((a, b) => {
		const aFolder = a.item === null
		const bFolder = b.item === null
		if (aFolder !== bFolder) return aFolder ? -1 : 1
		return a.name.localeCompare(b.name)
	})
	return { ...node, children, count: children.reduce((n, c) => n + c.count, 0) }
}

export function leaves<T>(nodes: TreeNode<T>[]): T[] {
	const out: T[] = []
	const walk = (node: TreeNode<T>) => {
		if (node.item !== null) out.push(node.item)
		node.children.forEach(walk)
	}
	nodes.forEach(walk)
	return out
}

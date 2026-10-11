import { leaves, type TreeNode } from '@/lib/files/tree'
import type { GitChange } from '@/lib/tauri/types'

export type ChangeWord = 'added' | 'changed' | 'removed' | 'conflict'

export function statusWord(status: string): ChangeWord {
	if (status === '?' || status === 'A') return 'added'
	if (status === 'D') return 'removed'
	return 'changed'
}

export function changeWord(change: GitChange): ChangeWord {
	return change.conflicted ? 'conflict' : statusWord(change.status)
}

export function changeMark(word: ChangeWord): string {
	return { added: '+', changed: '±', removed: '−', conflict: '!' }[word]
}

export function isContentPath(path: string): boolean {
	return path.startsWith('content/')
}

export function displayPath(path: string): string {
	return path.startsWith('overrides/') ? path.slice('overrides/'.length) : path
}

export function folderPathOf(node: TreeNode<GitChange>): string {
	const first = leaves([node])[0]
	return first && first.path !== displayPath(first.path) ? `overrides/${node.path}` : node.path
}

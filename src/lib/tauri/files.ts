import { call } from './call'
import type { FileContent, FileMatch, FsEntry, NbtNode, SearchOptions } from './types'

export function list(root: string, rel: string): Promise<FsEntry[]> {
	return call('fs_list', { root, rel })
}

export function read(root: string, rel: string): Promise<FileContent> {
	return call('fs_read', { root, rel })
}

export function write(root: string, rel: string, content: string): Promise<void> {
	return call('fs_write', { root, rel, content })
}

export function mkdir(root: string, rel: string): Promise<void> {
	return call('fs_mkdir', { root, rel })
}

export function remove(root: string, rel: string): Promise<void> {
	return call('fs_delete', { root, rel })
}

export function rename(root: string, from: string, to: string): Promise<void> {
	return call('fs_rename', { root, from, to })
}

export function readImage(root: string, rel: string): Promise<string> {
	return call('fs_read_image', { root, rel })
}

export function readNbt(root: string, rel: string): Promise<NbtNode> {
	return call('fs_read_nbt', { root, rel })
}

export function search(root: string, query: string, options: SearchOptions): Promise<FileMatch[]> {
	return call('search_files', { root, query, options })
}

export function replace(
	root: string,
	query: string,
	options: SearchOptions,
	replacement: string,
	only: string[],
): Promise<number> {
	return call('replace_in_files', { root, query, options, replacement, only })
}

export function index(root: string): Promise<string[]> {
	return call('fs_index', { root })
}

export function modConfigs(dir: string, key: string): Promise<string[]> {
	return call('mod_configs', { path: dir, key })
}

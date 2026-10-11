import { call } from './call'
import type { Manifest, PackState, PackwizInfo } from './types'

export function createPack(
	dir: string,
	name: string,
	minecraft: string,
	loader: string,
	loaderVersion: string | null,
): Promise<Manifest> {
	return call('create_pack', { path: dir, name, minecraft, loader, loaderVersion })
}

export function openPack(dir: string): Promise<PackState> {
	return call('open_pack', { path: dir })
}

export function readManifest(dir: string): Promise<Manifest> {
	return call('read_manifest', { path: dir })
}

export function pinLoaderVersion(dir: string): Promise<Manifest | null> {
	return call('pin_loader_version', { path: dir })
}

export function importPack(source: string, dest: string): Promise<PackState> {
	return call('import_pack', { src: source, dest })
}

export function clonePack(url: string, parent: string): Promise<PackState> {
	return call('clone_pack', { url, destParent: parent })
}

export function packwizInfo(dir: string): Promise<PackwizInfo | null> {
	return call('packwiz_info', { path: dir })
}

export function convertPackwiz(dir: string): Promise<PackState> {
	return call('convert_packwiz', { path: dir })
}

export function saveManifest(dir: string, manifest: Manifest): Promise<void> {
	return call('save_manifest', { path: dir, manifest })
}

export function setDatapackFolder(dir: string, folder: string): Promise<Manifest> {
	return call('set_datapack_folder', { path: dir, folder })
}

export function preservePatterns(dir: string): Promise<string[]> {
	return call('preserve_patterns', { path: dir })
}

export function preserveUpdate(dir: string, remove: string[], add: string[]): Promise<string[]> {
	return call('preserve_update', { path: dir, remove, add })
}

export function preserveReset(dir: string): Promise<string[]> {
	return call('preserve_reset', { path: dir })
}

export function packIcon(dir: string): Promise<string | null> {
	return call('pack_icon', { path: dir })
}

export function setPackIcon(dir: string, source: string | null): Promise<void> {
	return call('set_pack_icon', { path: dir, source })
}

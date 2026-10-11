import type { ProjectType } from '@/lib/tauri/types'

export const CONTENT_TYPES: ProjectType[] = ['mod', 'resourcepack', 'shader', 'datapack']

export const DATAPACK_FOLDERS: { folder: string; by: string }[] = [
	{ folder: 'datapacks', by: 'Default, Paxi' },
	{ folder: 'config/paxi/datapacks', by: 'Paxi' },
	{ folder: 'config/openloader/data', by: 'Open Loader' },
]

const CONTENT_DIRS: Record<string, ProjectType> = {
	mods: 'mod',
	resourcepacks: 'resourcepack',
	shaders: 'shader',
	datapacks: 'datapack',
}

export function contentPathType(path: string): ProjectType {
	return CONTENT_DIRS[path.split('/')[1] ?? ''] ?? 'mod'
}

export function versionLoader(type: ProjectType, loader: string | null): string | null {
	if (type === 'mod') return loader
	return type === 'datapack' ? 'datapack' : null
}

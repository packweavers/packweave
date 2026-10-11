import type { LockedMod, ProjectType, ProviderId } from '@/lib/tauri/types'

export type Platform = 'modrinth' | 'curseforge'

export const PLATFORMS: Platform[] = ['modrinth', 'curseforge']

const CURSEFORGE_CLASS: Record<ProjectType, string> = {
	mod: 'mc-mods',
	resourcepack: 'texture-packs',
	shader: 'shaders',
	datapack: 'data-packs',
}

export function curseforgePage(type: ProjectType, slug: string): string {
	return `https://www.curseforge.com/minecraft/${CURSEFORGE_CLASS[type]}/${slug}`
}

export function isPlatform(provider: ProviderId): provider is Platform {
	return provider === 'modrinth' || provider === 'curseforge'
}

function sourceId(mod: LockedMod, platform: Platform): string | null {
	if (mod.preferred === platform) return mod.projectId
	return mod.sources[platform]?.id ?? null
}

export function platformPage(mod: LockedMod, platform: Platform): string | null {
	const source = mod.sources[platform]
	if (!source) return null
	if (platform === 'modrinth') {
		const ref = source.slug || sourceId(mod, platform)
		return ref ? `https://modrinth.com/project/${ref}` : null
	}
	return source.slug ? curseforgePage(mod.projectType, source.slug) : null
}

export function hasSource(mod: LockedMod, provider: ProviderId, id: string): boolean {
	if (mod.preferred === provider && mod.projectId === id) return true
	const source = mod.sources[provider]
	return !!source && (source.id === id || source.slug === id)
}

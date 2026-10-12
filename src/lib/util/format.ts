import type { ProjectType, ProviderId } from '@/lib/tauri/types'

const SECOND = 1000
const MINUTE = 60 * SECOND
const HOUR = 60 * MINUTE
const DAY = 24 * HOUR
const WEEK = 7 * DAY
const MONTH = 30 * DAY
const YEAR = 365 * DAY

const UNITS: [number, string, string][] = [
	[YEAR, 'y', 'year'],
	[MONTH, 'mo', 'month'],
	[WEEK, 'w', 'week'],
	[DAY, 'd', 'day'],
	[HOUR, 'h', 'hour'],
	[MINUTE, 'm', 'minute'],
]

export function ago(time: number | string | Date, style: 'short' | 'long' = 'long'): string {
	const ms =
		time instanceof Date ? time.getTime() : typeof time === 'string' ? Date.parse(time) : time
	if (!Number.isFinite(ms)) return ''
	const diff = Math.max(0, Date.now() - ms)
	for (const [size, short, long] of UNITS) {
		if (diff >= size) {
			const n = Math.floor(diff / size)
			if (style === 'short') return `${n}${short}`
			if (long === 'day' && n === 1) return 'yesterday'
			return `${n} ${long}${n === 1 ? '' : 's'} ago`
		}
	}
	return style === 'short' ? 'now' : 'just now'
}

export function duration(ms: number): string {
	const total = Math.max(0, Math.floor(ms / 1000))
	const h = Math.floor(total / 3600)
	const m = Math.floor((total % 3600) / 60)
	const s = total % 60
	const pad = (n: number) => String(n).padStart(2, '0')
	return h > 0 ? `${h}:${pad(m)}:${pad(s)}` : `${m}:${pad(s)}`
}

export function compactCount(n: number): string {
	if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(n >= 10_000_000 ? 0 : 1)}M`
	if (n >= 1_000) return `${(n / 1_000).toFixed(n >= 10_000 ? 0 : 1)}k`
	return String(n)
}

export function bytes(n: number): string {
	if (n >= 1024 * 1024 * 1024) return `${(n / 1024 / 1024 / 1024).toFixed(1)} GB`
	if (n >= 1024 * 1024) return `${(n / 1024 / 1024).toFixed(1)} MB`
	if (n >= 1024) return `${Math.round(n / 1024)} KB`
	return `${n} B`
}

export function plural(n: number, one: string, many: string): string {
	return `${n} ${n === 1 ? one : many}`
}

export function listNames(names: string[], max = 2): string {
	if (names.length === 0) return ''
	if (names.length <= max) return names.join(names.length === 2 ? ' and ' : ', ')
	return `${names.slice(0, max).join(', ')} and ${names.length - max} more`
}

const LOADERS: Record<string, string> = {
	fabric: 'Fabric',
	quilt: 'Quilt',
	forge: 'Forge',
	neoforge: 'NeoForge',
	vanilla: 'Vanilla',
}

export function loaderName(loader: string): string {
	return LOADERS[loader] ?? loader
}

export function loaderVersionOptions(
	versions: string[],
	current: string | null,
): { value: string; label: string }[] {
	const options = versions.map((v, i) => ({ value: v, label: i === 0 ? `newest (${v})` : v }))
	if (current && !versions.includes(current)) options.push({ value: current, label: current })
	return options
}

export function gameLabel(minecraft: string, loader: string, loaderVersion: string | null): string {
	const loaderText = loaderVersion ? `${loaderName(loader)} ${loaderVersion}` : loaderName(loader)
	return loader === 'vanilla' ? minecraft : `${minecraft} · ${loaderText}`
}

const PROVIDERS: Record<ProviderId, string> = {
	modrinth: 'Modrinth',
	curseforge: 'CurseForge',
	url: 'URL',
	local: 'Local file',
}

export function providerName(id: ProviderId | null | undefined): string {
	return id ? PROVIDERS[id] : ''
}

const TYPES: Record<ProjectType, [string, string]> = {
	mod: ['Mod', 'Mods'],
	resourcepack: ['Resource pack', 'Resource packs'],
	shader: ['Shader', 'Shaders'],
	datapack: ['Data pack', 'Data packs'],
}

export function typeName(type: ProjectType, many = false): string {
	return TYPES[type][many ? 1 : 0]
}

export function sideDetail(clientSide: string, serverSide: string): string {
	const client = clientSide !== 'unsupported'
	const server = serverSide !== 'unsupported'
	if (client && server) {
		if (clientSide === 'optional' && serverSide === 'optional') {
			return 'Client and server, optional on both'
		}
		if (clientSide === 'optional') return 'Client and server, optional on client'
		if (serverSide === 'optional') return 'Client and server, optional on server'
		return 'Client and server'
	}
	if (client) return 'Client only'
	if (server) return 'Server only'
	return 'Neither'
}

export function sideName(clientSide: string, serverSide: string): string {
	const client = clientSide !== 'unsupported'
	const server = serverSide !== 'unsupported'
	if (client && server) return 'Both'
	if (client) return 'Client'
	if (server) return 'Server'
	return 'Neither'
}

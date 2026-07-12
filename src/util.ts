import dayjs from 'dayjs'
import relativeTime from 'dayjs/plugin/relativeTime'

dayjs.extend(relativeTime)

export function formatCount(n: number): string {
	if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`
	if (n >= 1_000) return `${(n / 1_000).toFixed(1)}K`
	return `${n}`
}

export function formatBytes(n: number): string {
	if (n >= 1_048_576) return `${(n / 1_048_576).toFixed(1)} MB`
	if (n >= 1024) return `${(n / 1024).toFixed(0)} KB`
	return `${n} B`
}

export function basename(path: string): string {
	const parts = path.split(/[/\\]/).filter(Boolean)
	return parts[parts.length - 1] ?? path
}

export function fromNow(iso: string): string {
	const d = dayjs(iso)
	return d.isValid() ? d.fromNow() : ''
}

export function fromNowMs(ms: number): string {
	const d = dayjs(ms)
	return d.isValid() ? d.fromNow() : ''
}

export function capitalize(s: string): string {
	return s.length ? s[0].toUpperCase() + s.slice(1) : s
}

const LOADER_LABELS: Record<string, string> = {
	neoforge: 'NeoForge',
	fabric: 'Fabric',
	quilt: 'Quilt',
	forge: 'Forge',
	vanilla: 'Vanilla',
}

export function loaderLabel(loader: string): string {
	return LOADER_LABELS[loader?.toLowerCase()] ?? capitalize(loader ?? '')
}

export const isMac =
	typeof navigator !== 'undefined' && /Mac|iPhone|iPad/i.test(navigator.userAgent)

export const GIT_EMPTY_TREE = '4b825dc642cb6eb9a060e54bf8d69288fbee4904'

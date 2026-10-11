import type { ItemChange, PackDiff } from '@/lib/tauri/types'
import { gameLabel } from '@/lib/util/format'

export type ChangelogFormat = 'markdown' | 'plain' | 'discord'

export interface ChangelogOptions {
	versions: boolean
	nestDependencies: boolean
	configs: boolean
	commits: boolean
	format: ChangelogFormat
}

export const DEFAULT_OPTIONS: ChangelogOptions = {
	versions: true,
	nestDependencies: true,
	configs: true,
	commits: false,
	format: 'markdown',
}

function heading(text: string, format: ChangelogFormat): string {
	if (format === 'markdown') return `### ${text}`
	if (format === 'discord') return `**${text}**`
	return text
}

function bullet(text: string, format: ChangelogFormat): string {
	return format === 'discord' ? `• ${text}` : `- ${text}`
}

function named(item: ItemChange, version: string | null, options: ChangelogOptions): string {
	return options.versions && version ? `${item.name} ${version}` : item.name
}

export function renderChangelog(
	diff: PackDiff,
	commits: string[],
	title: string | null,
	options: ChangelogOptions,
): string {
	const { format } = options
	const sections: [string, string[]][] = []
	const added = diff.items.filter((i) => i.kind === 'added' || i.kind === 'enabled')
	const removed = diff.items.filter((i) => i.kind === 'removed' || i.kind === 'disabled')
	const changed = diff.items.filter((i) => ['updated', 'provider', 'resided'].includes(i.kind))
	const addedNames = new Set(added.filter((i) => !i.dependency).map((i) => i.name))

	const nested = (item: ItemChange) =>
		options.nestDependencies && item.dependency && item.parents.some((p) => addedNames.has(p))

	sections.push([
		'Added',
		added
			.filter((i) => !nested(i))
			.map((item) => {
				const deps = added.filter((d) => nested(d) && d.parents.includes(item.name))
				const base = named(item, item.toVersion, options)
				return deps.length
					? `${base}, with ${deps.map((d) => named(d, d.toVersion, options)).join(', ')}`
					: base
			}),
	])

	const changes: string[] = []
	if (diff.env) {
		const env = diff.env
		changes.push(
			`${gameLabel(env.fromMinecraft, env.fromLoader, env.fromLoaderVersion)} → ${gameLabel(env.toMinecraft, env.toLoader, env.toLoaderVersion)}`,
		)
	}
	for (const item of changed) {
		if (item.kind === 'updated') {
			changes.push(
				options.versions
					? `${item.name} ${item.fromVersion ?? '?'} → ${item.toVersion ?? '?'}`
					: `${item.name} updated`,
			)
		} else if (item.kind === 'provider') {
			changes.push(`${item.name} now comes from ${item.toProvider ?? 'another source'}`)
		} else {
			changes.push(`${item.name} is now ${item.toSide ?? 'on another side'}`)
		}
	}
	sections.push(['Changed', changes])
	sections.push(['Removed', removed.map((i) => i.name)])
	if (options.configs) {
		sections.push([
			'Settings',
			diff.files.map(
				(f) =>
					`${f.path} ${f.status === 'added' ? 'added' : f.status === 'removed' ? 'removed' : 'changed'}`,
			),
		])
	}
	if (options.commits) sections.push(['Notes', commits])

	const parts: string[] = []
	if (title) {
		parts.push(
			format === 'markdown' ? `## ${title}` : format === 'discord' ? `__**${title}**__` : title,
		)
	}
	for (const [name, lines] of sections) {
		if (lines.length === 0) continue
		parts.push([heading(name, format), ...lines.map((l) => bullet(l, format))].join('\n'))
	}
	if (parts.length === (title ? 1 : 0)) parts.push('No pack changes.')
	return parts.join('\n\n') + '\n'
}

export function summarize(diff: PackDiff): string {
	const count = (kinds: string[]) => diff.items.filter((i) => kinds.includes(i.kind)).length
	const added = count(['added', 'enabled'])
	const changed = count(['updated', 'provider', 'resided']) + (diff.env ? 1 : 0)
	const removed = count(['removed', 'disabled'])
	return [
		added ? `${added} added` : '',
		changed ? `${changed} changed` : '',
		removed ? `${removed} removed` : '',
	]
		.filter(Boolean)
		.join(', ')
}

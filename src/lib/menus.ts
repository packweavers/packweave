import { platformPage, PLATFORMS, type Platform } from '@/lib/content/sources'
import { app } from '@/lib/state/app.svelte'
import { content } from '@/lib/state/content.svelte'
import { git } from '@/lib/state/git.svelte'
import { instance } from '@/lib/state/instance.svelte'
import { isPanelTab, layout, type PanelTab } from '@/lib/state/layout.svelte'
import type { MenuEntry } from '@/lib/state/menus.svelte'
import { preserve } from '@/lib/state/preserve.svelte'
import { tabs, type Doc } from '@/lib/state/tabs.svelte'
import { toasts } from '@/lib/state/toasts.svelte'
import { ws } from '@/lib/state/workspace.svelte'
import { addIgnorePatterns } from '@/lib/tauri/instance'
import type {
	IgnoreKind,
	ItemChange,
	LocalContent,
	LockedMod,
	SyncIgnored,
	Tag,
} from '@/lib/tauri/types'
import { openExternal, openInDefaultApp, openPackWindow, reveal } from '@/lib/tauri/windows'
import { providerName } from '@/lib/util/format'
import { join } from '@/lib/util/path'
import { fileManager } from '@/lib/util/platform'

import {
	deleteBranch,
	deleteRemoteBranch,
	mergeBranch,
	newBranch,
	openBranch,
	renameBranch,
	switchBranch,
} from './flows/branches'
import {
	deleteLocalFile,
	findSources,
	removeContent,
	toggleDisabled,
	updateContent,
} from './flows/content'
import { deleteFile, ignoreKindOf, openInFiles, resetIgnoreFile } from './flows/files'
import {
	addRemote,
	cherryPick,
	createTag,
	deleteTag,
	discard,
	discardContent,
	fetch,
	pull,
	push,
	resetTo,
	revertCommit,
} from './flows/git'
import { linkInstance, revealInstance, unlinkInstance } from './flows/instance'
import { closePack, newPack, openPackAt, openPackFolder } from './flows/pack'
import {
	closeDoc,
	closeOthers,
	closeToRight,
	movePanelToEditor,
	movePanelToNewWindow,
	moveToMainWindow,
	moveToNewWindow,
	moveToPanel,
} from './flows/tabs'

function copy(text: string, what: string): void {
	void navigator.clipboard
		.writeText(text)
		.then(() => toasts.show(`Copied ${what}`))
		.catch((error) => toasts.error(error))
}

function revealInPack(rel: string): void {
	if (ws.dir) void reveal(join(ws.dir, rel))
}

function openInPack(rel: string): void {
	if (ws.dir) void openInDefaultApp(join(ws.dir, rel))
}

function addIgnore(kind: IgnoreKind, rel: string, isDir: boolean): void {
	if (!ws.dir) return
	void addIgnorePatterns(ws.dir, kind, [isDir ? `${rel}/` : rel])
		.then(() => ws.changed())
		.catch((error) => toasts.error(error))
}

export function packMenu(): MenuEntry[] {
	const recents = app.prefs.recents.filter((r) => r.dir !== ws.mainDir).slice(0, 6)
	return [
		...(recents.length ? [{ heading: 'Recent packs' }] : []),
		...recents.map((r) => ({ label: r.name, run: () => void openPackWindow(r.dir, r.name) })),
		...(recents.length ? ['separator' as const] : []),
		{ label: 'Open pack…', command: 'pack.open', run: () => void openPackFolder() },
		{ label: 'New pack…', run: () => void newPack() },
		'separator',
		{ label: 'Pack settings', command: 'pack.settings', run: () => tabs.open('pack') },
		{ label: `Show in ${fileManager}`, run: () => revealInPack('') },
		'separator',
		{ label: 'Close pack', run: () => void closePack() },
	]
}

export function recentMenu(dir: string): MenuEntry[] {
	return [
		{ label: 'Open', run: () => void openPackAt(dir) },
		{ label: 'Open in new window', run: () => void openPackAt(dir, true) },
		{ label: `Show in ${fileManager}`, run: () => void reveal(dir) },
		'separator',
		{ label: 'Remove from this list', run: () => app.removeRecent(dir) },
	]
}

export function remoteMenu(): MenuEntry[] {
	if (!git.remoteName) {
		return [{ label: 'Add a remote…', run: () => void addRemote() }]
	}
	const ahead = git.status?.ahead ?? 0
	return [
		{
			label: ahead ? `Push ${ahead} to ${git.remoteName}` : `Push to ${git.remoteName}`,
			command: 'git.push',
			run: () => void push(),
		},
		{ label: `Pull from ${git.remoteName}`, command: 'git.pull', run: () => void pull() },
		{ label: 'Fetch', run: () => void fetch() },
	]
}

export function instanceMenu(): MenuEntry[] {
	if (!instance.bound) return [{ label: 'Link an instance…', run: () => void linkInstance() }]
	return [
		{
			label: 'Push to instance',
			command: 'instance.push',
			disabled: instance.toPush.length === 0,
			run: () => void instance.push(),
		},
		{
			label: 'Pull from instance',
			command: 'instance.pull',
			disabled: instance.toPull.length === 0,
			run: () => void instance.pull(),
		},
		{ label: 'Review', run: () => tabs.open('instance') },
		{ label: 'Rescan', run: () => void instance.scan() },
		'separator',
		{ label: 'Open folder', run: () => void revealInstance() },
		{ label: 'Change…', run: () => void linkInstance() },
		{ label: 'Unlink', run: () => void unlinkInstance() },
	]
}

export function tabMenu(doc: Doc): MenuEntry[] {
	return [
		{ label: 'Close', command: 'tab.close', disabled: doc.pinned, run: () => void closeDoc(doc) },
		{ label: 'Close others', run: () => void closeOthers(doc) },
		{ label: 'Close to the right', run: () => void closeToRight(doc) },
		'separator',
		{ label: doc.pinned ? 'Unpin' : 'Pin', run: () => tabs.pin(doc.id, !doc.pinned) },
		{
			label: 'Split right',
			command: 'editor.split',
			run: () => tabs.open(doc.kind, doc.arg, { side: true }),
		},
		{ label: 'Move to new window', run: () => void moveToNewWindow(doc) },
		...(ws.detached
			? [{ label: 'Move to main window', run: () => void moveToMainWindow(doc) }]
			: []),
		...(isPanelTab(doc.kind) && !ws.detached
			? [{ label: 'Move to panel', run: () => moveToPanel(doc) }]
			: []),
		...(doc.kind === 'file'
			? [
					'separator' as const,
					{ label: 'Copy path', run: () => copy(doc.arg, 'path') },
					{ label: `Show in ${fileManager}`, run: () => revealInPack(doc.arg) },
					{ label: 'Open in default app', run: () => openInPack(doc.arg) },
				]
			: []),
	]
}

export function tagMenu(tag: Tag): MenuEntry[] {
	return [
		{ label: 'Show in history', run: () => tabs.open('history', tag.commit) },
		{ label: 'Copy name', run: () => copy(tag.name, 'name') },
		'separator',
		{ label: 'Delete tag…', danger: true, run: () => void deleteTag(tag.name) },
	]
}

export function sourceMenu(mod: LockedMod, platform: Platform): MenuEntry[] {
	const page = platformPage(mod, platform)
	const preferred = mod.preferred === platform
	return [
		{
			label: `Show on ${providerName(platform)}`,
			disabled: !page,
			run: () => page && void openExternal(page),
		},
		{
			label: `Use ${providerName(platform)}`,
			disabled: preferred || !mod.sources[platform]?.versionNumber,
			run: () => void content.preferSource([mod.key], platform),
		},
		{ label: 'Pick version', run: () => tabs.open('mod', mod.key) },
		'separator',
		{
			label: `Remove ${providerName(platform)}`,
			danger: true,
			disabled: preferred,
			run: () => void content.removeSource(mod.key, platform),
		},
	]
}

export function contentChangeMenu(change: ItemChange): MenuEntry[] {
	const inPack = change.kind !== 'removed'
	return [
		{ label: 'Open', disabled: !inPack, run: () => tabs.open('mod', change.path) },
		{
			label: 'Open to the side',
			disabled: !inPack,
			run: () => tabs.open('mod', change.path, { side: true }),
		},
		{ label: 'Show changes', run: () => tabs.open('changes') },
		'separator',
		{ label: 'Copy name', run: () => copy(change.name, 'name') },
		{ label: 'Copy path', run: () => copy(change.path, 'path') },
		'separator',
		change.kind === 'removed'
			? { label: 'Restore', run: () => void discardContent([change], false) }
			: { label: 'Discard…', danger: true, run: () => void discardContent([change]) },
	]
}

export function panelMenu(tab: PanelTab): MenuEntry[] {
	return [
		{ label: 'Move to new window', run: () => void movePanelToNewWindow(tab) },
		{ label: 'Move to editor', run: () => movePanelToEditor(tab) },
		'separator',
		{ label: 'Close panel', command: 'layout.panel', run: () => layout.togglePanel() },
	]
}

export function contentMenu(ids: string[]): MenuEntry[] {
	const mods = ids.map((id) => content.find(id)).filter((m) => !!m)
	if (mods.length === 0) return []
	const one = mods.length === 1 ? mods[0] : null
	const update = one ? content.updates[one.key] : null
	const updatable = mods.filter((m) => content.updates[m.key]).length
	const allDisabled = mods.every((m) => m.disabled)
	const allPinned = mods.every((m) => m.pinned)
	const allUnbundled = mods.every((m) => m.noBundle)
	const entries: MenuEntry[] = []
	if (one) {
		entries.push(
			{ label: 'Open', run: () => tabs.open('mod', one.key) },
			{ label: 'Open to the side', run: () => tabs.open('mod', one.key, { side: true }) },
			'separator',
		)
	}
	if (update) {
		entries.push({
			label: `Update to ${update.versionNumber}`,
			command: 'content.update',
			run: () => void updateContent(ids),
		})
	} else if (updatable > 0) {
		entries.push({
			label: `Update ${updatable}`,
			command: 'content.update',
			run: () => void updateContent(ids),
		})
	}
	if (one) entries.push({ label: 'Pick version', run: () => tabs.open('mod', one.key) })
	entries.push(
		{ label: allDisabled ? 'Enable' : 'Disable', run: () => void toggleDisabled(ids) },
		{
			label: allPinned ? 'Unpin version' : 'Pin version',
			run: () => void content.setPinned(ids, !allPinned),
		},
		{
			label: allUnbundled ? 'Bundle in exports' : "Don't bundle in exports",
			run: () => void content.setBundled(ids, allUnbundled),
		},
	)
	if (one) {
		entries.push(
			one.dependencyType === 'dependency'
				? { label: 'Keep in the pack', run: () => void content.setExplicit(one.key, true) }
				: {
						label: 'Make it a dependency',
						run: () => void content.setExplicit(one.key, false),
					},
		)
		for (const platform of PLATFORMS) {
			if (platform === one.preferred) continue
			if (one.sources[platform]) {
				entries.push({
					label: `Use ${providerName(platform)} instead`,
					disabled: !one.sources[platform]?.versionNumber,
					run: () => void content.preferSource([one.key], platform),
				})
			} else if (app.providerReady(platform)) {
				entries.push({
					label: `Find on ${providerName(platform)}…`,
					run: () => void findSources([one.key], platform),
				})
			}
		}
		entries.push(
			'separator',
			{ label: 'Copy name', run: () => copy(one.name, 'name') },
			{ label: 'Copy project ID', run: () => copy(one.projectId, 'project ID') },
		)
		for (const platform of PLATFORMS) {
			const page = platformPage(one, platform)
			if (page) {
				entries.push({
					label: `Show on ${providerName(platform)}`,
					run: () => void openExternal(page),
				})
			}
		}
	}
	if (!one) {
		const platforms: MenuEntry[] = []
		for (const platform of PLATFORMS) {
			const switchable = mods.filter(
				(m) => m.preferred !== platform && !!m.sources[platform]?.versionNumber,
			)
			if (switchable.length > 0) {
				platforms.push({
					label: `Use ${providerName(platform)} for ${switchable.length}`,
					run: () =>
						void content.preferSource(
							switchable.map((m) => m.key),
							platform,
						),
				})
			}
			const missing = mods.filter((m) => !m.sources[platform])
			if (missing.length > 0 && app.providerReady(platform)) {
				platforms.push({
					label: `Find ${missing.length} on ${providerName(platform)}…`,
					run: () =>
						void findSources(
							missing.map((m) => m.key),
							platform,
						),
				})
			}
		}
		if (platforms.length > 0) entries.push('separator', ...platforms)
	}
	if (instance.bound) {
		entries.push(
			'separator',
			syncIgnoreEntry(mods.map((m) => ({ key: `mod:${m.key}`, label: m.name }))),
		)
	}
	entries.push('separator', {
		label: 'Remove',
		command: 'content.remove',
		danger: true,
		run: () => void removeContent(ids),
	})
	return entries
}

function syncIgnoreEntry(targets: SyncIgnored[]): MenuEntry {
	const rules = targets.map((t) => instance.ignoredEntryFor(t.key))
	if (rules.some((rule) => rule === null)) {
		return { label: `Ignore for ${instance.name}`, run: () => void instance.ignore(targets) }
	}
	const keys = [...new Set(rules.map((rule) => rule!.key))]
	const outer = keys.length === 1 && keys[0] !== targets[0].key ? rules[0]!.label : null
	return {
		label: outer ? `Stop ignoring ${outer}` : `Stop ignoring for ${instance.name}`,
		run: () => void instance.unignore(keys),
	}
}

export function localMenu(file: LocalContent): MenuEntry[] {
	return [
		{ label: `Show in ${fileManager}`, run: () => revealInPack(`overrides/${file.relPath}`) },
		{ label: 'Copy file name', run: () => copy(file.filename, 'file name') },
		...(instance.bound
			? [
					'separator' as const,
					syncIgnoreEntry([{ key: `jar:${file.relPath}`, label: file.filename }]),
				]
			: []),
		'separator',
		{ label: 'Delete', danger: true, run: () => void deleteLocalFile(file) },
	]
}

export function fileMenu(rel: string, isDir: boolean, inFiles = false): MenuEntry[] {
	const changes = git.changes
		.filter((c) => c.path === rel || (isDir && c.path.startsWith(`${rel}/`)))
		.map((c) => c.path)
	const instanceRel = rel.startsWith('overrides/') ? rel.slice('overrides/'.length) : null
	const entries: MenuEntry[] = []
	if (isDir && !inFiles) {
		entries.push({ label: 'Open', run: () => void openInFiles(rel, true) }, 'separator')
	}
	if (!isDir) {
		entries.push(
			{ label: 'Open', run: () => tabs.open('file', rel) },
			{ label: 'Open to the side', run: () => tabs.open('file', rel, { side: true }) },
			'separator',
		)
	}
	entries.push(
		{ label: 'Copy path', run: () => copy(rel, 'path') },
		{ label: `Show in ${fileManager}`, run: () => revealInPack(rel) },
		...(isDir ? [] : [{ label: 'Open in default app', run: () => openInPack(rel) }]),
	)
	const ignoreKind = isDir ? null : ignoreKindOf(rel)
	if (ignoreKind) {
		entries.push({ label: 'Reset to defaults…', run: () => void resetIgnoreFile(ignoreKind) })
	}
	if (changes.length > 0) {
		entries.push('separator', {
			label: 'Discard changes…',
			danger: true,
			run: () => void discard(changes),
		})
	}
	entries.push(
		'separator',
		{ label: 'Add to .gitignore', run: () => addIgnore('git', rel, isDir) },
		{ label: 'Exclude from Exports', run: () => addIgnore('export', rel, isDir) },
	)
	if (instanceRel) {
		const target = isDir ? `${instanceRel}/` : instanceRel
		const unlocked = preserve.fileUnlocked(target)
		entries.push('separator', {
			label: unlocked ? 'Lock from player edits' : 'Unlock for player edits',
			run: () => void preserve.setFile(target, !unlocked),
		})
	}
	if (instanceRel && instance.binding) {
		entries.push(
			{
				label: `Show in ${instance.name}`,
				run: () => void reveal(join(instance.binding!.gameDir, instanceRel)),
			},
			syncIgnoreEntry([
				isDir
					? { key: `file:${instanceRel}/`, label: `${instanceRel}/` }
					: { key: `file:${instanceRel}`, label: instanceRel },
			]),
		)
	}
	entries.push('separator', { label: 'Delete', danger: true, run: () => void deleteFile(rel) })
	return entries
}

export function branchMenu(name: string): MenuEntry[] {
	const current = name === git.branch
	const remote = git.isRemote(name)
	const tree = git.worktreeFor(name)
	const open = tree ? git.windowFor(tree.path) : undefined
	return [
		...(current
			? []
			: [
					{
						label: open ? 'Go to window' : 'Switch to it',
						run: () => void (open ? openBranch(name) : switchBranch(name)),
					},
				]),
		{ label: 'Open in new window', command: 'branch.openWindow', run: () => void openBranch(name) },
		'separator',
		...(current
			? []
			: [
					{
						label: `Compare with ${git.branch}`,
						run: () => tabs.open('compare', `${git.branch}..${name}`),
					},
					{ label: `Merge into ${git.branch}…`, run: () => void mergeBranch(name) },
				]),
		{ label: 'New branch from here…', run: () => void newBranch(name) },
		'separator',
		...(remote
			? [
					{
						label: `Delete on ${name.slice(0, name.indexOf('/'))}…`,
						danger: true,
						run: () => void deleteRemoteBranch(name),
					},
				]
			: [
					{ label: 'Rename…', run: () => void renameBranch(name) },
					{
						label: 'Delete branch…',
						danger: true,
						disabled: current,
						run: () => void deleteBranch(name),
					},
				]),
	]
}

export function commitMenu(hash: string): MenuEntry[] {
	return [
		{ label: 'New branch here…', run: () => void newBranch(hash) },
		{ label: 'Tag here…', run: () => void createTag(hash) },
		{ label: `Cherry-pick onto ${git.branch}`, run: () => void cherryPick(hash) },
		{ label: 'Revert this commit', run: () => void revertCommit(hash) },
		'separator',
		{ label: 'Copy hash', run: () => copy(hash, 'hash') },
		{ label: `Reset ${git.branch} to here…`, danger: true, run: () => void resetTo(hash) },
	]
}

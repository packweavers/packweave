import {
	ArrowDownToLine,
	ArrowUpFromLine,
	Blocks,
	Bug,
	FileText,
	GitBranch,
	History,
	Plus,
	RefreshCw,
	Rocket,
	Search,
	Settings,
} from '@lucide/svelte'
import { getCurrentWindow } from '@tauri-apps/api/window'

import { newBranch } from '@/lib/flows/branches'
import {
	openAddContent,
	pasteList,
	removeContent,
	toggleDisabled,
	updateAll,
	updateContent,
} from '@/lib/flows/content'
import { fetch, initRepo, pull, push, stashChanges } from '@/lib/flows/git'
import { linkInstance } from '@/lib/flows/instance'
import {
	clonePack,
	closePack,
	importPack,
	newPack,
	openPackFolder,
	packFromInstance,
} from '@/lib/flows/pack'
import { closeDoc, moveToMainWindow, moveToNewWindow } from '@/lib/flows/tabs'
import { pausedWhileRunning } from '@/lib/instance/items'
import { registerCommands, type Command } from '@/lib/state/commands.svelte'
import { content } from '@/lib/state/content.svelte'
import { files } from '@/lib/state/files.svelte'
import { git } from '@/lib/state/git.svelte'
import { instance } from '@/lib/state/instance.svelte'
import { layout } from '@/lib/state/layout.svelte'
import { palette } from '@/lib/state/palette.svelte'
import { panels } from '@/lib/state/panels.svelte'
import { selection } from '@/lib/state/selection.svelte'
import { tabs } from '@/lib/state/tabs.svelte'
import { ws } from '@/lib/state/workspace.svelte'
import { openSettingsWindow } from '@/lib/tauri/windows'

function packOpen(): boolean {
	return ws.isOpen
}

function fullWindow(): boolean {
	return ws.isOpen && !ws.detached
}

function selectedContent(): string[] {
	const doc = tabs.active
	if (!doc) return []
	if (doc.kind === 'mod') return [doc.arg]
	if (doc.kind === 'content') return selection.of(doc.id).filter((id) => content.find(id))
	return []
}

const general: Command[] = [
	{
		id: 'palette.open',
		title: 'Search everything',
		group: 'General',
		icon: Search,
		run: () => palette.show(),
	},
	{
		id: 'palette.goto',
		title: 'Go to file or content',
		group: 'General',
		when: packOpen,
		run: () => palette.show('', 'goto'),
	},
	{
		id: 'palette.commands',
		title: 'Run a command',
		group: 'General',
		run: () => palette.show('>'),
	},
	{ id: 'pack.open', title: 'Open pack…', group: 'General', run: openPackFolder },
	{ id: 'pack.new', title: 'New pack…', group: 'General', run: newPack },
	{ id: 'pack.import', title: 'Import a pack…', group: 'General', run: () => importPack() },
	{ id: 'pack.clone', title: 'Clone a pack from Git…', group: 'General', run: clonePack },
	{
		id: 'pack.fromInstance',
		title: 'Make a pack from an instance…',
		group: 'General',
		run: packFromInstance,
	},
	{ id: 'pack.close', title: 'Close pack', group: 'General', when: packOpen, run: closePack },
	{
		id: 'app.settings',
		title: 'Settings',
		group: 'General',
		icon: Settings,
		run: () => openSettingsWindow(),
	},
	{
		id: 'pack.settings',
		title: 'Pack settings',
		group: 'General',
		when: packOpen,
		run: () => tabs.open('pack'),
	},
	{
		id: 'edit.undo',
		title: 'Undo',
		group: 'General',
		when: packOpen,
		whileTyping: false,
		run: () => content.undo(),
	},
	{
		id: 'edit.redo',
		title: 'Redo',
		group: 'General',
		when: packOpen,
		whileTyping: false,
		run: () => content.redo(),
	},
]

const windows: Command[] = [
	{
		id: 'branch.openWindow',
		title: 'Open branch in new window…',
		group: 'Windows and tabs',
		icon: GitBranch,
		when: () => packOpen() && git.isRepo,
		run: () => palette.show('#', 'git', true),
	},
	{
		id: 'branch.new',
		title: 'New branch…',
		group: 'Windows and tabs',
		icon: GitBranch,
		when: () => packOpen() && git.isRepo,
		run: () => newBranch(),
	},
	{
		id: 'tab.close',
		title: 'Close tab',
		group: 'Windows and tabs',
		when: () => tabs.active !== null,
		run: () => {
			if (tabs.active) void closeDoc(tabs.active)
		},
	},
	{
		id: 'tab.reopen',
		title: 'Reopen closed tab',
		group: 'Windows and tabs',
		when: packOpen,
		run: () => tabs.reopen(),
	},
	{
		id: 'tab.next',
		title: 'Next tab',
		group: 'Windows and tabs',
		when: packOpen,
		run: () => tabs.cycle(1),
	},
	{
		id: 'tab.previous',
		title: 'Previous tab',
		group: 'Windows and tabs',
		when: packOpen,
		run: () => tabs.cycle(-1),
	},
	...Array.from({ length: 9 }, (_, i) => ({
		id: `tab.goto${i + 1}`,
		title: `Go to tab ${i + 1}`,
		group: 'Windows and tabs',
		when: packOpen,
		run: () => tabs.goto(i + 1),
	})),
	{
		id: 'editor.split',
		title: 'Split right',
		group: 'Windows and tabs',
		when: () => tabs.active !== null,
		run: () => tabs.split(),
	},
	{
		id: 'tab.newWindow',
		title: 'Move tab to new window',
		group: 'Windows and tabs',
		when: () => !!tabs.active,
		run: () => {
			if (tabs.active) void moveToNewWindow(tabs.active)
		},
	},
	{
		id: 'tab.mainWindow',
		title: 'Move tab to main window',
		group: 'Windows and tabs',
		when: () => ws.detached && !!tabs.active,
		run: () => {
			if (tabs.active) void moveToMainWindow(tabs.active)
		},
	},
	{
		id: 'window.close',
		title: 'Close window',
		group: 'Windows and tabs',
		run: () => getCurrentWindow().close(),
	},
]

const views: Command[] = [
	{
		id: 'view.content',
		title: 'Content',
		group: 'Views',
		icon: Blocks,
		when: fullWindow,
		run: () => layout.showView('content'),
	},
	{
		id: 'view.files',
		title: 'Files',
		group: 'Views',
		icon: FileText,
		when: fullWindow,
		run: () => layout.showView('files'),
	},
	{
		id: 'view.search',
		title: 'Search',
		group: 'Views',
		icon: Search,
		when: fullWindow,
		run: () => layout.showView('search'),
	},
	{
		id: 'view.debug',
		title: 'Debug',
		group: 'Views',
		icon: Bug,
		when: fullWindow,
		run: () => layout.showView('debug'),
	},
	{
		id: 'view.git',
		title: 'Git',
		group: 'Views',
		icon: GitBranch,
		when: fullWindow,
		run: () => layout.showView('git'),
	},
	{
		id: 'panel.problems',
		title: 'Problems',
		group: 'Views',
		when: fullWindow,
		run: () => panels.open('problems'),
	},
	{
		id: 'panel.output',
		title: 'Output',
		group: 'Views',
		when: fullWindow,
		run: () => panels.open('output'),
	},
	{
		id: 'panel.gamelog',
		title: 'Game log',
		group: 'Views',
		when: fullWindow,
		run: () => panels.open('gamelog'),
	},
	{
		id: 'layout.sidebar',
		title: 'Toggle sidebar',
		group: 'Views',
		when: fullWindow,
		run: () => layout.toggleSidebar(),
	},
	{
		id: 'layout.panel',
		title: 'Toggle bottom panel',
		group: 'Views',
		when: fullWindow,
		run: () => layout.togglePanel(),
	},
	{
		id: 'layout.inspector',
		title: 'Toggle inspector',
		group: 'Views',
		when: packOpen,
		run: () => layout.toggleInspector(),
	},
]

const contentCommands: Command[] = [
	{
		id: 'content.add',
		title: 'Add content',
		group: 'Content',
		icon: Plus,
		when: packOpen,
		run: () => openAddContent(),
	},
	{
		id: 'content.paste',
		title: 'Paste a list…',
		group: 'Content',
		when: packOpen,
		run: pasteList,
	},
	{
		id: 'content.update',
		title: 'Update selected',
		group: 'Content',
		when: () => selectedContent().some((id) => content.updates[id]),
		run: () => updateContent(selectedContent()),
	},
	{
		id: 'content.updateAll',
		title: 'Update all mods',
		group: 'Content',
		when: () => packOpen() && content.updateCount > 0,
		run: updateAll,
	},
	{
		id: 'content.remove',
		title: 'Remove selected',
		group: 'Content',
		whileTyping: false,
		when: () => selectedContent().length > 0,
		run: () => removeContent(selectedContent()),
	},
	{
		id: 'content.disable',
		title: 'Disable or enable selected',
		group: 'Content',
		when: () => selectedContent().length > 0,
		run: () => toggleDisabled(selectedContent()),
	},
	{
		id: 'content.check',
		title: 'Check for updates',
		group: 'Content',
		icon: RefreshCw,
		when: packOpen,
		run: () => content.checkUpdates(true),
	},
	{
		id: 'content.resolve',
		title: 'Resolve again',
		group: 'Content',
		when: packOpen,
		run: () => content.resolve(true),
	},
]

const fileCommands: Command[] = [
	{
		id: 'file.save',
		title: 'Save',
		group: 'Files',
		when: () => tabs.active?.kind === 'file' || tabs.active?.kind === 'pack',
		run: () => {
			const doc = tabs.active
			if (doc?.kind === 'file') void files.save(doc.arg)
		},
	},
]

const gitCommands: Command[] = [
	{
		id: 'git.init',
		title: 'Start using Git',
		group: 'Git',
		when: () => packOpen() && !git.isRepo,
		run: initRepo,
	},
	{
		id: 'git.changes',
		title: 'Show changes',
		group: 'Git',
		when: () => packOpen() && git.isRepo,
		run: () => tabs.open('changes'),
	},
	{
		id: 'git.history',
		title: 'Show history',
		group: 'Git',
		icon: History,
		when: () => packOpen() && git.isRepo,
		run: () => tabs.open('history'),
	},
	{
		id: 'git.branches',
		title: 'Show branches',
		group: 'Git',
		when: () => packOpen() && git.isRepo,
		run: () => tabs.open('branches'),
	},
	{
		id: 'git.push',
		title: 'Push to origin',
		group: 'Git',
		icon: ArrowUpFromLine,
		when: () => packOpen() && git.remoteName !== null,
		run: push,
	},
	{
		id: 'git.pull',
		title: 'Pull from origin',
		group: 'Git',
		icon: ArrowDownToLine,
		when: () => packOpen() && git.remoteName !== null,
		run: pull,
	},
	{
		id: 'git.fetch',
		title: 'Fetch',
		group: 'Git',
		when: () => packOpen() && git.remoteName !== null,
		run: fetch,
	},
	{
		id: 'git.stash',
		title: 'Stash changes…',
		group: 'Git',
		when: () => packOpen() && git.changes.length > 0,
		run: stashChanges,
	},
]

const instanceCommands: Command[] = [
	{
		id: 'instance.push',
		title: 'Push to instance',
		group: 'Instance',
		when: () =>
			instance.toPush.some((item) => !instance.running || !pausedWhileRunning(item, 'push')),
		run: () => instance.push(),
	},
	{
		id: 'instance.pull',
		title: 'Pull from instance',
		group: 'Instance',
		when: () =>
			instance.toPull.some((item) => !instance.running || !pausedWhileRunning(item, 'pull')),
		run: () => instance.pull(),
	},
	{
		id: 'instance.review',
		title: 'Review instance changes',
		group: 'Instance',
		when: () => instance.bound,
		run: () => tabs.open('instance'),
	},
	{
		id: 'instance.rescan',
		title: 'Rescan the instance',
		group: 'Instance',
		when: () => instance.bound,
		run: () => instance.scan(),
	},
	{
		id: 'instance.link',
		title: 'Link an instance…',
		group: 'Instance',
		when: packOpen,
		run: linkInstance,
	},
]

const releaseCommands: Command[] = [
	{
		id: 'release.open',
		title: 'Export & Release',
		group: 'Release',
		icon: Rocket,
		when: packOpen,
		run: () => tabs.open('release'),
	},
	{
		id: 'changelog.open',
		title: 'Changelog',
		group: 'Release',
		when: () => packOpen() && git.isRepo,
		run: () => tabs.open('changelog'),
	},
]

export function registerAll(): void {
	registerCommands([
		...general,
		...windows,
		...views,
		...contentCommands,
		...fileCommands,
		...gitCommands,
		...instanceCommands,
		...releaseCommands,
	])
}

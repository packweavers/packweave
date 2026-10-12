import { getVersion } from '@tauri-apps/api/app'
import {
	Menu,
	MenuItem,
	PredefinedMenuItem,
	Submenu,
	type AboutMetadata,
} from '@tauri-apps/api/menu'
import { getCurrentWindow } from '@tauri-apps/api/window'

import { openPackAt } from '@/lib/flows/pack'
import { app } from '@/lib/state/app.svelte'
import {
	comboFor,
	commandById,
	isAvailable,
	runCommand,
	type Command,
} from '@/lib/state/commands.svelte'
import { tasks } from '@/lib/state/tasks.svelte'
import { openExternal, openSettingsWindow } from '@/lib/tauri/windows'

type Predefined =
	| 'Copy'
	| 'Cut'
	| 'Paste'
	| 'SelectAll'
	| 'Undo'
	| 'Redo'
	| 'Minimize'
	| 'Maximize'
	| 'Fullscreen'
	| 'Hide'
	| 'HideOthers'
	| 'ShowAll'
	| 'CloseWindow'
	| 'Quit'
	| 'Services'
	| 'BringAllToFront'

type Entry =
	| 'separator'
	| { predefined: Predefined }
	| { about: AboutMetadata }
	| { label: string; command: string; key?: boolean }
	| { label: string; run: () => unknown; keyOf?: string; enabled?: boolean }
	| { label: string; items: Entry[]; role?: 'window' | 'help' }

type Item = MenuItem | Submenu | PredefinedMenuItem

interface Built {
	items: Item[]
	tracked: { item: MenuItem; command: Command }[]
	roles: [Submenu, 'window' | 'help'][]
	created: Item[]
}

const LINKS: [string, string][] = [
	['packweave Website', 'https://packweave.com'],
	['Discord', 'https://discord.gg/VZBgts9jCj'],
	['Report an Issue', 'https://github.com/packweavers/packweave/issues'],
	["What's New", 'https://github.com/packweavers/packweave/releases'],
]

let generation = 0
let installed: Item[] = []
let menu: Menu | null = null
let stopEnabled: (() => void) | null = null

function accelerator(combo: string | null): string | undefined {
	if (!combo) return undefined
	return combo
		.split('+')
		.map((part) => (part === 'Mod' ? 'CmdOrCtrl' : part))
		.join('+')
}

async function menuItem(
	text: string,
	keys: string | null,
	enabled: boolean,
	action: () => void,
): Promise<MenuItem> {
	const shortcut = accelerator(keys)
	try {
		return await MenuItem.new({ text, accelerator: shortcut, enabled, action })
	} catch (error) {
		if (!shortcut) throw error
		return MenuItem.new({ text, enabled, action })
	}
}

async function buildEntry(entry: Entry, out: Built): Promise<Item> {
	let item: Item
	if (entry === 'separator') {
		item = await PredefinedMenuItem.new({ item: 'Separator' })
	} else if ('predefined' in entry) {
		item = await PredefinedMenuItem.new({ item: entry.predefined })
	} else if ('about' in entry) {
		item = await PredefinedMenuItem.new({ item: { About: entry.about } })
	} else if ('items' in entry) {
		const children: Item[] = []
		for (const child of entry.items) children.push(await buildEntry(child, out))
		const submenu = await Submenu.new({
			text: entry.label,
			items: children,
			enabled: children.length > 0,
		})
		if (entry.role) out.roles.push([submenu, entry.role])
		item = submenu
	} else if ('command' in entry) {
		const id = entry.command
		const command = commandById(id)
		const made = await menuItem(
			entry.label,
			entry.key === false ? null : comboFor(id),
			command ? isAvailable(command) : false,
			() => void runCommand(id),
		)
		if (command) out.tracked.push({ item: made, command })
		item = made
	} else {
		const run = entry.run
		item = await menuItem(
			entry.label,
			entry.keyOf ? comboFor(entry.keyOf) : null,
			entry.enabled ?? true,
			() => void run(),
		)
	}
	out.created.push(item)
	return item
}

async function install(spec: () => Promise<Entry[]>): Promise<void> {
	const mine = ++generation
	const out: Built = { items: [], tracked: [], roles: [], created: [] }
	try {
		for (const entry of await spec()) out.items.push(await buildEntry(entry, out))
		if (mine !== generation) {
			for (const item of out.created) void item.close()
			return
		}
		const next = await Menu.new({ items: out.items })
		await next.setAsAppMenu()
		for (const [submenu, role] of out.roles) {
			if (role === 'window') await submenu.setAsWindowsMenuForNSApp()
			else await submenu.setAsHelpMenuForNSApp()
		}
		stopEnabled?.()
		stopEnabled = $effect.root(() => {
			for (const { item, command } of out.tracked) {
				$effect(() => {
					void item.setEnabled(isAvailable(command))
				})
			}
		})
		const old = [...installed, ...(menu ? [menu] : [])]
		installed = out.created
		menu = next
		for (const resource of old) void resource.close()
	} catch (error) {
		tasks.fail('Building the menu bar', error)
	}
}

function closeTabOrWindow(): void {
	if (!runCommand('tab.close')) void getCurrentWindow().close()
}

function helpMenu(): Entry {
	return {
		label: 'Help',
		role: 'help',
		items: LINKS.map(([label, url]) => ({ label, run: () => openExternal(url) })),
	}
}

async function packMenu(): Promise<Entry[]> {
	const version = await getVersion()
	return [
		{
			label: 'packweave',
			items: [
				{ about: { name: 'packweave', version } },
				'separator',
				{ label: 'Check for Updates…', run: () => openSettingsWindow('updates') },
				'separator',
				{ label: 'Settings…', command: 'app.settings' },
				{ label: 'Pack Settings…', command: 'pack.settings' },
				'separator',
				{ predefined: 'Services' },
				'separator',
				{ predefined: 'Hide' },
				{ predefined: 'HideOthers' },
				{ predefined: 'ShowAll' },
				'separator',
				{ predefined: 'Quit' },
			],
		},
		{
			label: 'File',
			items: [
				{ label: 'New Pack…', command: 'pack.new' },
				{ label: 'Open Pack…', command: 'pack.open' },
				{
					label: 'Open Recent',
					items: app.prefs.recents.map((recent) => ({
						label: recent.name,
						run: () => openPackAt(recent.dir),
					})),
				},
				'separator',
				{ label: 'Import a Pack…', command: 'pack.import' },
				{ label: 'Clone a Pack from Git…', command: 'pack.clone' },
				{ label: 'Make a Pack from an Instance…', command: 'pack.fromInstance' },
				'separator',
				{ label: 'Save', command: 'file.save' },
				'separator',
				{ label: 'Close Tab', run: closeTabOrWindow, keyOf: 'tab.close' },
				{ label: 'Reopen Closed Tab', command: 'tab.reopen' },
				{ label: 'Close Pack', command: 'pack.close' },
				{ label: 'Close Window', command: 'window.close' },
			],
		},
		{
			label: 'Edit',
			items: [
				{ predefined: 'Undo' },
				{ predefined: 'Redo' },
				'separator',
				{ predefined: 'Cut' },
				{ predefined: 'Copy' },
				{ predefined: 'Paste' },
				{ predefined: 'SelectAll' },
				'separator',
				{ label: 'Search Everything', command: 'palette.open' },
				{ label: 'Go to File or Content', command: 'palette.goto' },
				{ label: 'Run a Command', command: 'palette.commands' },
			],
		},
		{
			label: 'View',
			items: [
				{ label: 'Content', command: 'view.content' },
				{ label: 'Files', command: 'view.files' },
				{ label: 'Search', command: 'view.search' },
				{ label: 'Git', command: 'view.git' },
				{ label: 'Debug', command: 'view.debug' },
				'separator',
				{ label: 'Problems', command: 'panel.problems' },
				{ label: 'Output', command: 'panel.output' },
				{ label: 'Game Log', command: 'panel.gamelog' },
				'separator',
				{ label: 'Toggle Sidebar', command: 'layout.sidebar' },
				{ label: 'Toggle Bottom Panel', command: 'layout.panel' },
				{ label: 'Toggle Inspector', command: 'layout.inspector' },
				'separator',
				{ label: 'Split Right', command: 'editor.split' },
				'separator',
				{ predefined: 'Fullscreen' },
			],
		},
		{
			label: 'Content',
			items: [
				{ label: 'Add Content…', command: 'content.add' },
				{ label: 'Paste a List…', command: 'content.paste' },
				'separator',
				{ label: 'Update Selected', command: 'content.update' },
				{ label: 'Update All Mods', command: 'content.updateAll' },
				{ label: 'Disable or Enable Selected', command: 'content.disable' },
				{ label: 'Remove Selected', command: 'content.remove', key: false },
				'separator',
				{ label: 'Check for Mod Updates', command: 'content.check' },
				{ label: 'Resolve Again', command: 'content.resolve' },
			],
		},
		{
			label: 'Git',
			items: [
				{ label: 'Commit…', command: 'git.compose' },
				{ label: 'Push', command: 'git.push' },
				{ label: 'Pull', command: 'git.pull' },
				{ label: 'Fetch', command: 'git.fetch' },
				{ label: 'Stash Changes…', command: 'git.stash' },
				'separator',
				{ label: 'Show Changes', command: 'git.changes' },
				{ label: 'Show History', command: 'git.history' },
				{ label: 'Show Branches', command: 'git.branches' },
				'separator',
				{ label: 'New Branch…', command: 'branch.new' },
				{ label: 'Open Branch in New Window…', command: 'branch.openWindow' },
				'separator',
				{ label: 'Start Using Git', command: 'git.init' },
			],
		},
		{
			label: 'Instance',
			items: [
				{ label: 'Push to Instance', command: 'instance.push' },
				{ label: 'Pull from Instance', command: 'instance.pull' },
				{ label: 'Review Changes', command: 'instance.review' },
				{ label: 'Rescan', command: 'instance.rescan' },
				'separator',
				{ label: 'Link an Instance…', command: 'instance.link' },
			],
		},
		{
			label: 'Release',
			items: [
				{ label: 'Export & Release…', command: 'release.open' },
				{ label: 'Changelog', command: 'changelog.open' },
			],
		},
		{
			label: 'Window',
			role: 'window',
			items: [
				{ predefined: 'Minimize' },
				{ predefined: 'Maximize' },
				'separator',
				{ label: 'Next Tab', command: 'tab.next' },
				{ label: 'Previous Tab', command: 'tab.previous' },
				{ label: 'Move Tab to New Window', command: 'tab.newWindow' },
				{ label: 'Move Tab to Main Window', command: 'tab.mainWindow' },
				'separator',
				{ predefined: 'BringAllToFront' },
			],
		},
		helpMenu(),
	]
}

async function settingsMenu(): Promise<Entry[]> {
	const version = await getVersion()
	return [
		{
			label: 'packweave',
			items: [
				{ about: { name: 'packweave', version } },
				'separator',
				{ predefined: 'Services' },
				'separator',
				{ predefined: 'Hide' },
				{ predefined: 'HideOthers' },
				{ predefined: 'ShowAll' },
				'separator',
				{ predefined: 'Quit' },
			],
		},
		{
			label: 'Edit',
			items: [
				{ predefined: 'Undo' },
				{ predefined: 'Redo' },
				'separator',
				{ predefined: 'Cut' },
				{ predefined: 'Copy' },
				{ predefined: 'Paste' },
				{ predefined: 'SelectAll' },
			],
		},
		{
			label: 'Window',
			role: 'window',
			items: [
				{ predefined: 'Minimize' },
				{ predefined: 'Maximize' },
				'separator',
				{ predefined: 'CloseWindow' },
				'separator',
				{ predefined: 'BringAllToFront' },
			],
		},
		helpMenu(),
	]
}

function follow(spec: () => Promise<Entry[]>, rebuildOn: () => unknown): () => void {
	const win = getCurrentWindow()
	let focused = false
	let first = true
	const stopFocus = win.onFocusChanged(({ payload }) => {
		focused = payload
		if (payload) void install(spec)
	})
	void win.isFocused().then((yes) => {
		focused = yes
		if (yes) void install(spec)
	})
	const stopRebuild = $effect.root(() => {
		$effect(() => {
			rebuildOn()
			if (first) {
				first = false
				return
			}
			if (focused) void install(spec)
		})
	})
	return () => {
		void stopFocus.then((unlisten) => unlisten())
		stopRebuild()
		stopEnabled?.()
		stopEnabled = null
	}
}

export function followPackMenu(): () => void {
	return follow(packMenu, () => [
		JSON.stringify(app.prefs.keymap),
		app.prefs.recents.map((recent) => `${recent.dir}\n${recent.name}`).join('\n'),
	])
}

export function followSettingsMenu(): () => void {
	return follow(settingsMenu, () => null)
}

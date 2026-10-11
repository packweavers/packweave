import {
	Blocks,
	Bug,
	FileDiff,
	FileText,
	GitBranch,
	GitCompareArrows,
	GitMerge,
	History,
	Monitor,
	NotebookText,
	Package,
	Rocket,
	ScrollText,
	Settings,
	SquareDot,
	SquareTerminal,
	TriangleAlert,
} from '@lucide/svelte'
import type { Component } from 'svelte'

import AddInspector from '@/inspectors/AddInspector.svelte'
import BranchInspector from '@/inspectors/BranchInspector.svelte'
import ChangelogInspector from '@/inspectors/ChangelogInspector.svelte'
import CommitInspector from '@/inspectors/CommitInspector.svelte'
import ContentInspector from '@/inspectors/ContentInspector.svelte'
import FileInspector from '@/inspectors/FileInspector.svelte'
import ModPageInspector from '@/inspectors/ModPageInspector.svelte'
import ReleaseInspector from '@/inspectors/ReleaseInspector.svelte'
import SyncInspector from '@/inspectors/SyncInspector.svelte'
import { content } from '@/lib/state/content.svelte'
import { git } from '@/lib/state/git.svelte'
import { instance } from '@/lib/state/instance.svelte'
import type { Doc, DocKind } from '@/lib/state/tabs.svelte'
import type { ProjectType } from '@/lib/tauri/types'
import { typeName } from '@/lib/util/format'
import { basename } from '@/lib/util/path'

import AddDoc from './AddDoc.svelte'
import BisectDoc from './BisectDoc.svelte'
import BranchesDoc from './BranchesDoc.svelte'
import ChangelogDoc from './ChangelogDoc.svelte'
import ChangesDoc from './ChangesDoc.svelte'
import CompareDoc from './CompareDoc.svelte'
import ConflictsDoc from './ConflictsDoc.svelte'
import ContentDoc from './ContentDoc.svelte'
import DiffDoc from './DiffDoc.svelte'
import FileDoc from './FileDoc.svelte'
import HistoryDoc from './HistoryDoc.svelte'
import InstanceDoc from './InstanceDoc.svelte'
import ModDoc from './ModDoc.svelte'
import PackDoc from './PackDoc.svelte'
import PanelDoc from './PanelDoc.svelte'
import ReleaseDoc from './ReleaseDoc.svelte'

type DocComponent = Component<{ doc: Doc }>

interface DocType {
	component: DocComponent
	inspector?: DocComponent
	title: (doc: Doc) => string
	icon: Component
}

export const DOCS: Record<DocKind, DocType> = {
	content: {
		component: ContentDoc,
		inspector: ContentInspector,
		title: (doc) => typeName((doc.arg || 'mod') as ProjectType, true),
		icon: Blocks,
	},
	mod: {
		component: ModDoc,
		inspector: ModPageInspector,
		title: (doc) => content.find(doc.arg)?.name ?? doc.arg,
		icon: SquareDot,
	},
	add: { component: AddDoc, inspector: AddInspector, title: () => 'Add content', icon: Package },
	bisect: { component: BisectDoc, title: () => 'Find a broken mod', icon: Bug },
	file: {
		component: FileDoc,
		inspector: FileInspector,
		title: (doc) => basename(doc.arg),
		icon: FileText,
	},
	diff: {
		component: DiffDoc,
		title: (doc) => `${basename(doc.arg.slice(doc.arg.indexOf(':') + 1))} @ ${doc.arg.slice(0, 7)}`,
		icon: FileDiff,
	},
	changes: { component: ChangesDoc, title: () => 'Changes', icon: FileDiff },
	history: {
		component: HistoryDoc,
		inspector: CommitInspector,
		title: () => 'History',
		icon: History,
	},
	branches: {
		component: BranchesDoc,
		inspector: BranchInspector,
		title: () => 'Branches',
		icon: GitBranch,
	},
	compare: {
		component: CompareDoc,
		title: (doc) => {
			const [base, other] = doc.arg.split('..')
			return `${other} → ${base}`
		},
		icon: GitCompareArrows,
	},
	conflicts: { component: ConflictsDoc, title: () => 'Conflicts', icon: GitMerge },
	changelog: {
		component: ChangelogDoc,
		inspector: ChangelogInspector,
		title: () => 'Changelog',
		icon: NotebookText,
	},
	instance: {
		component: InstanceDoc,
		inspector: SyncInspector,
		title: () => instance.name || 'Instance',
		icon: Monitor,
	},
	release: {
		component: ReleaseDoc,
		inspector: ReleaseInspector,
		title: () => 'Export & Release',
		icon: Rocket,
	},
	pack: { component: PackDoc, title: () => 'Pack settings', icon: Settings },
	problems: { component: PanelDoc, title: () => 'Problems', icon: TriangleAlert },
	output: { component: PanelDoc, title: () => 'Output', icon: SquareTerminal },
	gamelog: { component: PanelDoc, title: () => 'Game log', icon: ScrollText },
}

export function docTitle(doc: Doc): string {
	return DOCS[doc.kind].title(doc)
}

export function docIcon(doc: Doc): Component {
	return DOCS[doc.kind].icon
}

export function docNeedsYou(doc: Doc): boolean {
	if (doc.kind === 'conflicts') return git.conflicts.length > 0
	if (doc.kind === 'file') return git.conflicts.includes(doc.arg)
	return false
}

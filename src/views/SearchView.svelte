<script lang="ts">
	import ConfirmDialog from '@/dialogs/ConfirmDialog.svelte'
	import { pathHints } from '@/lib/files/paths'
	import { displayPath } from '@/lib/git/changes'
	import { content } from '@/lib/state/content.svelte'
	import { dialogs } from '@/lib/state/dialogs.svelte'
	import { files } from '@/lib/state/files.svelte'
	import { git } from '@/lib/state/git.svelte'
	import { instance } from '@/lib/state/instance.svelte'
	import { tabs } from '@/lib/state/tabs.svelte'
	import { tasks } from '@/lib/state/tasks.svelte'
	import { toasts } from '@/lib/state/toasts.svelte'
	import { ws } from '@/lib/state/workspace.svelte'
	import { errorText } from '@/lib/tauri/call'
	import { replace, search } from '@/lib/tauri/files'
	import type { FileMatch, SearchOptions } from '@/lib/tauri/types'
	import { reveal } from '@/lib/tauri/windows'
	import { plural } from '@/lib/util/format'
	import { join, sameDir } from '@/lib/util/path'
	import { focusOnMount } from '@/ui/focus'
	import PathField from '@/ui/PathField.svelte'

	import SideSection from './SideSection.svelte'

	type Scope = 'files' | 'mods' | 'instance' | 'branches'

	interface BranchMatches {
		branch: string
		folder: string
		matches: FileMatch[]
	}

	let query = $state('')
	let replacement = $state('')
	let options = $state<SearchOptions>({
		caseSensitive: false,
		wholeWord: false,
		regex: false,
		include: '',
	})
	let scopes = $state<Record<Scope, boolean>>({
		files: true,
		mods: true,
		instance: false,
		branches: false,
	})
	let fileMatches = $state<FileMatch[]>([])
	let instanceMatches = $state<FileMatch[]>([])
	let branchMatches = $state<BranchMatches[]>([])
	let error = $state<string | null>(null)
	let searching = $state(false)
	let generation = 0

	const modMatches = $derived(
		scopes.mods && query.trim().length >= 2
			? content.mods.filter((m) => m.name.toLowerCase().includes(query.trim().toLowerCase()))
			: [],
	)
	const byFile = $derived(group(fileMatches))
	const total = $derived(
		fileMatches.length +
			modMatches.length +
			instanceMatches.length +
			branchMatches.reduce((n, b) => n + b.matches.length, 0),
	)

	async function onlyIn(text: string): Promise<string[]> {
		const cut = text.lastIndexOf(',') + 1
		const head = text.slice(0, cut) + (cut > 0 && !text.slice(cut).startsWith(' ') ? ' ' : '')
		const folders = (await files.gamePaths()).filter((p) => p.endsWith('/'))
		return pathHints(folders, text.slice(cut)).map((hint) => head + hint)
	}

	function group(matches: FileMatch[]): [string, FileMatch[]][] {
		const map = new Map<string, FileMatch[]>()
		for (const m of matches) map.set(m.path, [...(map.get(m.path) ?? []), m])
		return [...map.entries()]
	}

	$effect(() => {
		void query
		void $state.snapshot(options)
		void $state.snapshot(scopes)
		const timer = setTimeout(() => void run(), 250)
		return () => clearTimeout(timer)
	})

	async function searchBranches(
		dir: string,
		text: string,
		opts: SearchOptions,
	): Promise<BranchMatches[]> {
		const others = git.worktrees.filter((w) => !sameDir(w.path, dir))
		return Promise.all(
			others.map(async (w) => ({
				branch: w.branch ?? 'detached',
				folder: w.path,
				matches: await search(w.path, text, opts).catch((error) => {
					toasts.error(error)
					return []
				}),
			})),
		)
	}

	async function run(): Promise<void> {
		const id = ++generation
		const dir = ws.dir
		const text = query
		const opts = $state.snapshot(options)
		const gameDir = instance.binding?.gameDir
		error = null
		if (!dir || text.trim().length < 2) {
			fileMatches = []
			instanceMatches = []
			branchMatches = []
			searching = false
			return
		}
		searching = true
		try {
			const [inPack, inInstance, inBranches] = await Promise.all([
				scopes.files ? search(dir, text, opts) : [],
				scopes.instance && gameDir ? search(gameDir, text, opts) : [],
				scopes.branches ? searchBranches(dir, text, opts) : [],
			])
			if (id !== generation) return
			fileMatches = inPack
			instanceMatches = inInstance
			branchMatches = inBranches
		} catch (e) {
			if (id === generation) error = errorText(e)
		} finally {
			if (id === generation) searching = false
		}
	}

	function openMatch(match: FileMatch, keep: boolean): void {
		files.reveal = { rel: match.path, line: match.line }
		tabs.open('file', match.path, { preview: !keep })
	}

	function highlight(match: FileMatch): { text: string; hit: boolean }[] {
		const chars = [...match.text]
		const parts: { text: string; hit: boolean }[] = []
		let at = 0
		for (const [start, end] of match.ranges) {
			if (start > at) parts.push({ text: chars.slice(at, start).join(''), hit: false })
			parts.push({ text: chars.slice(start, end).join(''), hit: true })
			at = end
		}
		if (at < chars.length) parts.push({ text: chars.slice(at).join(''), hit: false })
		return parts
	}

	async function replaceAll(): Promise<void> {
		const dir = ws.dir
		if (!dir || fileMatches.length === 0) return
		const paths = [...new Set(fileMatches.map((m) => m.path))]
		const ok = await dialogs.open(ConfirmDialog, {
			title: `Replace in ${plural(paths.length, 'file', 'files')}?`,
			message: `${fileMatches.length} matches of "${query}" become "${replacement}".`,
			confirm: 'Replace all',
		})
		if (!ok) return
		try {
			const count = await tasks.run('Replacing', () =>
				replace(dir, query, $state.snapshot(options), replacement, paths),
			)
			for (const path of paths) {
				if (files.buffers[path] && !files.isDirty(path)) void files.reload(path)
			}
			ws.changed()
			toasts.show(`Replaced ${count}`)
			void run()
		} catch (e) {
			toasts.error(e)
		}
	}
</script>

{#snippet lines(matches: FileMatch[], onopen: (m: FileMatch, keep: boolean) => void)}
	{#each matches as match (`${match.path}:${match.line}`)}
		<button
			class="match"
			onclick={() => onopen(match, false)}
			ondblclick={() => onopen(match, true)}
		>
			<span class="line mono small">{match.line}</span>
			<span class="text mono">
				{#each highlight(match) as part, i (i)}<span class:hit={part.hit}>{part.text}</span>{/each}
			</span>
		</button>
	{/each}
{/snippet}

<div class="bar view-header"><span>Search</span></div>
<div class="form">
	<div class="input-row">
		<input
			class="field grow"
			bind:value={query}
			placeholder="Search"
			spellcheck="false"
			use:focusOnMount
		/>
		<button
			class="toggle mono"
			class:on={options.caseSensitive}
			aria-label="Match case"
			onclick={() => (options.caseSensitive = !options.caseSensitive)}>Aa</button
		>
		<button
			class="toggle mono"
			class:on={options.wholeWord}
			aria-label="Whole word"
			onclick={() => (options.wholeWord = !options.wholeWord)}>W</button
		>
		<button
			class="toggle mono"
			class:on={options.regex}
			aria-label="Regular expression"
			onclick={() => (options.regex = !options.regex)}>.*</button
		>
	</div>
	<div class="input-row">
		<input class="field grow" bind:value={replacement} placeholder="Replace" spellcheck="false" />
		<button class="btn" disabled={fileMatches.length === 0} onclick={replaceAll}>Replace all</button
		>
	</div>
	<div class="scopes">
		<button class="chip" class:on={scopes.files} onclick={() => (scopes.files = !scopes.files)}
			>Files</button
		>
		<button class="chip" class:on={scopes.mods} onclick={() => (scopes.mods = !scopes.mods)}
			>Mods</button
		>
		{#if instance.bound}
			<button
				class="chip"
				class:on={scopes.instance}
				onclick={() => (scopes.instance = !scopes.instance)}>Instance folder</button
			>
		{/if}
		{#if git.worktrees.length > 1}
			<button
				class="chip"
				class:on={scopes.branches}
				onclick={() => (scopes.branches = !scopes.branches)}>Other branches</button
			>
		{/if}
	</div>
	<PathField bind:value={options.include} placeholder="Only in, like config/" suggest={onlyIn} />
</div>

<div class="scroll results">
	{#if error}
		<div class="quiet field-error">{error}</div>
	{:else if query.trim().length >= 2}
		<div class="quiet small">
			{searching ? 'Searching…' : `${total} ${total === 1 ? 'result' : 'results'}`}
		</div>
	{/if}
	{#if byFile.length > 0}
		<SideSection title="Files" count={fileMatches.length}>
			{#each byFile as [path, matches] (path)}
				<div class="file mono small">{displayPath(path)}</div>
				{@render lines(matches, openMatch)}
			{/each}
		</SideSection>
	{/if}
	{#if modMatches.length > 0}
		<SideSection title="Mods" count={modMatches.length}>
			{#each modMatches as mod (mod.key)}
				<button class="match" onclick={() => tabs.open('mod', mod.key, { preview: true })}>
					<span class="text">{mod.name}</span>
				</button>
			{/each}
		</SideSection>
	{/if}
	{#if instanceMatches.length > 0 && instance.binding}
		<SideSection title={instance.name} count={instanceMatches.length}>
			{#each group(instanceMatches) as [path, matches] (path)}
				<div class="file mono small">{path}</div>
				{@render lines(matches, (m) => void reveal(join(instance.binding!.gameDir, m.path)))}
			{/each}
		</SideSection>
	{/if}
	{#each branchMatches.filter((b) => b.matches.length > 0) as found (found.folder)}
		<SideSection title={found.branch} count={found.matches.length}>
			{#each group(found.matches) as [path, matches] (path)}
				<div class="file mono small">{displayPath(path)}</div>
				{@render lines(matches, (m) => void reveal(join(found.folder, m.path)))}
			{/each}
		</SideSection>
	{/each}
</div>

<style>
	.form {
		flex: none;
		display: flex;
		flex-direction: column;
		gap: 6px;
		padding: 8px;
		border-bottom: 1px solid var(--line);
	}

	.input-row {
		display: flex;
		gap: 1px;
	}

	.grow {
		flex: 1;
		min-width: 0;
	}

	.toggle {
		width: 28px;
		display: inline-grid;
		place-items: center;
		font-size: 11px;
		background: var(--band);
	}

	.toggle.on {
		background: var(--selected);
		font-weight: 600;
	}

	.scopes {
		display: flex;
		flex-wrap: wrap;
		gap: 1px;
	}

	.quiet {
		padding: 6px 14px;
	}

	.file {
		padding: 4px 14px 2px;
		font-weight: 600;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.match {
		width: 100%;
		display: flex;
		gap: 8px;
		min-height: 24px;
		padding: 2px 14px 2px 22px;
		text-align: left;
	}

	.match:hover {
		background: var(--hover);
	}

	.line {
		flex: 0 0 28px;
		text-align: right;
	}

	.text {
		flex: 1;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.hit {
		background: var(--hit-bg);
	}
</style>

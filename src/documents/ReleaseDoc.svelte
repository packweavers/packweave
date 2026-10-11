<script lang="ts">
	import { onMount } from 'svelte'

	import { summarize } from '@/lib/content/changelog'
	import { openIgnoreFile } from '@/lib/flows/files'
	import { exportAll, onGithub, publish, publishes } from '@/lib/flows/release'
	import { app } from '@/lib/state/app.svelte'
	import { git } from '@/lib/state/git.svelte'
	import { release } from '@/lib/state/release.svelte'
	import { tabs, type Doc } from '@/lib/state/tabs.svelte'
	import { tasks } from '@/lib/state/tasks.svelte'
	import { toasts } from '@/lib/state/toasts.svelte'
	import { ws } from '@/lib/state/workspace.svelte'
	import {
		checkPackDistToken,
		packDistToken,
		savePackDistToken,
		secretStatus,
	} from '@/lib/tauri/app'
	import { compare, githubAccess, latestTag, packUrl } from '@/lib/tauri/git'
	import { includePackweaver } from '@/lib/tauri/release'
	import type { Channel, GithubAccess, PackDiff } from '@/lib/tauri/types'
	import { openExternal, openSettingsWindow } from '@/lib/tauri/windows'
	import { githubRepo } from '@/lib/util/text'
	import { pending } from '@/ui/pending'
	import Segmented from '@/ui/Segmented.svelte'

	interface Props {
		doc: Doc
	}

	let {}: Props = $props()

	const main = $derived(ws.mainDir ?? ws.dir ?? '')
	const prefs = $derived(app.pack(main).release)
	const typedRepo = $derived(prefs.repo.trim())
	const parsedRepo = $derived(githubRepo(typedRepo))
	const typedPublish = $derived(prefs.publishRepo.trim())
	const parsedPublish = $derived(githubRepo(typedPublish))

	let tag = $state<string | null>(null)
	let access = $state<GithubAccess | null>(null)
	let checking = $state(false)
	let rawUrl = $state<string | null>(null)
	let diff = $state<PackDiff | null>(null)
	let tokens = $state({ modrinth: false, curseforge: false })
	let distToken = $state('')
	let savedDistToken = $state('')
	let tokenWorks = $state<boolean | null>(null)
	let ownPublishAccess = $state<GithubAccess | null>(null)
	let publishChecking = $state(false)
	let accessRun = 0
	let publishRun = 0
	let tokenRun = 0

	const badRepo = $derived(typedRepo !== '' && parsedRepo === null)
	const repo = $derived(badRepo ? null : (access?.repo ?? null))
	const repoName = $derived(repo?.split('/')[1] ?? 'the repository')
	const badPublish = $derived(typedPublish !== '' && parsedPublish === null)
	const publishAccess = $derived(typedPublish === '' ? access : ownPublishAccess)
	const publishBusy = $derived(typedPublish === '' ? checking : publishChecking)
	const publishTarget = $derived(badPublish ? null : (publishAccess?.repo ?? null))
	const selfUpdating = $derived(prefs.updates !== 'none')
	const github = $derived(onGithub(prefs))

	function bump(version: string): string {
		const parts = version.split('.').map((p) => Number(p))
		if (parts.some((p) => Number.isNaN(p))) return version
		parts[parts.length - 1] += 1
		return parts.join('.')
	}

	onMount(() => {
		const dir = ws.dir
		if (!dir) return
		void latestTag(dir)
			.then((t) => {
				tag = t
				if (release.version) return
				const current = ws.manifest?.version ?? '1.0.0'
				release.version = t === `v${current}` ? bump(current) : current
				release.channel = ws.manifest?.channel ?? 'release'
			})
			.catch(tasks.or('Reading the latest release tag', undefined))
		void packUrl(dir)
			.then((url) => (rawUrl = url))
			.catch(tasks.or('Finding the pack link', undefined))
		void secretStatus(['modrinth_token', 'curseforge_token'])
			.then(([modrinth, curseforge]) => (tokens = { modrinth, curseforge }))
			.catch(tasks.or('Checking the platform tokens', undefined))
		void packDistToken(dir)
			.then((saved) => {
				savedDistToken = saved ?? ''
				distToken = savedDistToken
			})
			.catch(tasks.or('Reading the update token', undefined))
	})

	$effect(() => {
		const dir = ws.dir
		const wanted = parsedRepo
		if (!dir || badRepo) return
		const run = ++accessRun
		checking = true
		void githubAccess(dir, wanted)
			.then((found) => {
				if (run === accessRun) access = found
			})
			.catch((error) => {
				if (run === accessRun) access = null
				tasks.fail('Checking the GitHub repository', error)
			})
			.finally(() => {
				if (run === accessRun) checking = false
			})
	})

	$effect(() => {
		const dir = ws.dir
		const wanted = parsedPublish
		ownPublishAccess = null
		if (!dir || !wanted) return
		const run = ++publishRun
		publishChecking = true
		void githubAccess(dir, wanted)
			.then((found) => {
				if (run === publishRun) ownPublishAccess = found
			})
			.catch((error) => {
				if (run === publishRun) ownPublishAccess = null
				tasks.fail('Checking the GitHub repository', error)
			})
			.finally(() => {
				if (run === publishRun) publishChecking = false
			})
	})

	$effect(() => {
		const dir = ws.dir
		const saved = savedDistToken
		const target = repo
		tokenWorks = null
		if (!dir || !saved || !target) return
		const run = ++tokenRun
		void checkPackDistToken(dir, target)
			.then((works) => {
				if (run === tokenRun) tokenWorks = works
			})
			.catch(tasks.or('Checking the update token', undefined))
	})

	$effect(() => {
		const dir = ws.dir
		const since = tag
		void git.head
		if (!dir || !since) return
		void compare(dir, since, 'HEAD')
			.then((c) => (diff = c.pack))
			.catch((error) => {
				diff = null
				tasks.fail('Comparing with the last release', error)
			})
	})

	function update(change: (r: typeof prefs) => void): void {
		app.updatePack(main, (pack) => change(pack.release))
	}

	function setUpdates(value: typeof prefs.updates): void {
		update((r) => (r.updates = value))
		const dir = ws.dir
		if (value !== 'url' || !dir) return
		void includePackweaver(dir)
			.then(() => ws.changed())
			.catch(tasks.or('Adding packweaver to the pack', undefined))
	}

	function setRepo(value: string): void {
		const typed = value.trim()
		update((r) => (r.repo = typed === '' ? '' : (githubRepo(typed) ?? typed)))
	}

	function setPublishRepo(value: string): void {
		const typed = value.trim()
		update((r) => (r.publishRepo = typed === '' ? '' : (githubRepo(typed) ?? typed)))
	}

	async function saveDistToken(): Promise<void> {
		const dir = ws.dir
		const token = distToken.trim()
		if (!dir || token === savedDistToken) return
		try {
			await savePackDistToken(dir, token)
			savedDistToken = token
			toasts.show(token ? 'Token saved' : 'Token removed')
		} catch (error) {
			toasts.error(error)
		}
	}

	function createToken(): void {
		if (!repo) return
		const [owner, name] = repo.split('/')
		const query = new URLSearchParams({
			name: `${ws.name} updates`.slice(0, 40),
			description: `Lets ${ws.name} players download updates from ${repo}. Under Repository access, pick only ${name}.`,
			target_name: owner,
			expires_in: 'none',
			contents: 'read',
		})
		void openExternal(`https://github.com/settings/personal-access-tokens/new?${query}`)
	}

	function hintFor(
		bad: boolean,
		target: string | null,
		found: GithubAccess | null,
		busy: boolean,
	): string | null {
		if (bad) return 'Use owner/name, like packweavers/my-pack'
		if (!target) return 'Enter the repository the releases go to'
		if (busy || !found) return null
		if (found.visibility === 'missing') {
			return found.signedIn
				? 'Not found, or your GitHub sign-in can’t see it'
				: 'Not found. If it’s private, sign in to GitHub to check'
		}
		if (found.visibility === 'unknown') return 'Couldn’t reach GitHub to check it'
		return null
	}

	const repoHint = $derived(hintFor(badRepo, repo, access, checking))
	const publishHint = $derived(hintFor(badPublish, publishTarget, publishAccess, publishBusy))
	const elsewhere = $derived(
		prefs.updates === 'github' &&
			!!publishTarget &&
			!!repo &&
			publishTarget.toLowerCase() !== repo.toLowerCase(),
	)

	const tokenHint = $derived.by(() => {
		if (savedDistToken && tokenWorks === false) {
			return `This token can’t read ${repoName}. Give it read access to ${repoName}`
		}
		return ''
	})

	const head = $derived((git.branches?.list ?? []).find((b) => b.current)?.head ?? null)
	const plan = $derived({
		version: release.version.trim() || '1.0.0',
		channel: release.channel,
		notes: release.notes,
	})
</script>

{#snippet visibility(found: GithubAccess | null, busy: boolean, bad: boolean)}
	{#if busy}
		<span class="spinner"></span>
	{:else if found?.visibility === 'public'}
		<span class="tag">public</span>
	{:else if found?.visibility === 'private'}
		<span class="tag changed">private</span>
	{:else if found?.visibility === 'missing' || bad}
		<span class="tag removed">not found</span>
	{/if}
{/snippet}

<div class="bar doc-toolbar">
	<span class="doc-title">Release</span>
	<span>from {git.branch ?? ws.name}</span>
	<span class="spacer"></span>
	<button class="btn" onclick={pending(() => exportAll(plan))}>Export…</button>
	{#if publishes(prefs)}
		<button class="btn primary" onclick={pending(() => publish(plan))}>Publish…</button>
	{/if}
</div>

<div class="scroll page">
	<section>
		<div class="band bar"><span class="n">1</span>Version</div>
		<div class="line">
			<input class="field mono version" bind:value={release.version} spellcheck="false" />
			{#if tag}<span class="small">last {tag}</span>{/if}
			<span class="spacer"></span>
			<Segmented
				options={[
					{ value: 'release' as Channel, label: 'Release' },
					{ value: 'beta' as Channel, label: 'Beta' },
					{ value: 'alpha' as Channel, label: 'Alpha' },
				]}
				bind:value={release.channel}
			/>
		</div>
	</section>

	<section>
		<div class="band bar">
			<span class="n">2</span>Changes
			<span class="spacer"></span>
			<button class="btn" onclick={() => tabs.open('changelog')}>Edit</button>
		</div>
		<div class="line small">
			{git.branch ?? ''}{head ? ` at ${head}` : ''}{diff
				? ` · ${summarize(diff) || 'no pack changes'}`
				: ''}
		</div>
		<textarea class="field-area notes" rows="6" bind:value={release.notes} placeholder="What's new"
		></textarea>
	</section>

	<section>
		<div class="band bar"><span class="n">3</span>Updates</div>
		<div class="line">
			<Segmented
				options={[
					{ value: 'none', label: 'Off' },
					{ value: 'github', label: 'GitHub releases' },
					{ value: 'url', label: 'Your own URL' },
				]}
				value={prefs.updates}
				onchange={setUpdates}
			/>
		</div>
		{#if prefs.updates === 'github'}
			<div class="line">
				<span class="label">Repository</span>
				<input
					class="field mono grow"
					value={prefs.repo}
					placeholder={access?.origin ?? 'owner/name'}
					class:prefilled={!!access?.origin}
					spellcheck="false"
					onchange={(e) => setRepo(e.currentTarget.value)}
				/>
				{@render visibility(access, checking, badRepo)}
			</div>
			{#if repoHint}
				<div class="line small hint">
					{repoHint}
					{#if access && !access.signedIn && access.visibility === 'missing'}
						<button class="btn small" onclick={() => void openSettingsWindow('accounts')}
							>Sign in…</button
						>
					{/if}
				</div>
			{/if}
			<div class="line">
				<span class="label">Read-only token</span>
				<input
					class="field mono grow"
					type="password"
					bind:value={distToken}
					placeholder="github_pat_…"
					spellcheck="false"
					onchange={saveDistToken}
				/>
				{#if savedDistToken && tokenWorks === false}
					<span class="tag removed">✗ can’t read</span>
				{:else if savedDistToken}
					<span class="tag added">✓ saved</span>
				{:else if access?.visibility === 'private'}
					<span class="tag removed">required</span>
				{:else if access?.visibility === 'public'}
					<span class="tag">optional</span>
				{/if}
				<button class="btn" disabled={!repo} onclick={createToken}>Create token…</button>
			</div>
			<div class="line small hint">{tokenHint}</div>
		{:else if prefs.updates === 'url'}
			<div class="line">
				<span class="label">URL</span>
				<input
					class="field mono grow"
					value={prefs.url}
					placeholder={rawUrl ?? 'https://example.com/my-pack'}
					class:prefilled={!!rawUrl}
					spellcheck="false"
					onchange={(e) => update((r) => (r.url = e.currentTarget.value.trim()))}
				/>
			</div>
			<div class="line small hint">
				{prefs.url || !rawUrl
					? 'Players sync to whatever this URL serves'
					: 'Empty uses the default branch. Players get every push'}
			</div>
		{/if}
	</section>

	<section>
		<div class="band bar">
			<span class="n">4</span>Exports
			<span class="spacer"></span>
			<button class="btn" onclick={pending(() => openIgnoreFile('export'))}>Excluded files</button>
		</div>
		<label class="line check"
			><input
				type="checkbox"
				checked={prefs.exports.mrpack}
				onchange={(e) => update((r) => (r.exports.mrpack = e.currentTarget.checked))}
			/>Modrinth pack <span class="mono small">.mrpack</span>
			{#if selfUpdating}<span class="tag info">self-updating</span>{/if}</label
		>
		<label class="line check"
			><input
				type="checkbox"
				checked={prefs.exports.curseforge}
				onchange={(e) => update((r) => (r.exports.curseforge = e.currentTarget.checked))}
			/>CurseForge pack <span class="mono small">.zip</span>
			{#if selfUpdating}<span class="tag info">self-updating</span>{/if}</label
		>
		<label class="line check"
			><input
				type="checkbox"
				checked={prefs.exports.instance}
				onchange={(e) => update((r) => (r.exports.instance = e.currentTarget.checked))}
			/>Prism / MultiMC instance <span class="mono small">.zip</span>
			{#if selfUpdating}<span class="tag info">self-updating</span>{/if}</label
		>
		<label class="line check"
			><input
				type="checkbox"
				checked={prefs.exports.server}
				onchange={(e) => update((r) => (r.exports.server = e.currentTarget.checked))}
			/>Server pack <span class="mono small">.mrpack</span></label
		>
		<label class="line check"
			><input
				type="checkbox"
				checked={prefs.exports.client}
				onchange={(e) => update((r) => (r.exports.client = e.currentTarget.checked))}
			/>Client pack <span class="mono small">.mrpack</span></label
		>
		<div class="line small hint">
			Overrides, like local mods and data packs (if not a .mrpack), go into exports as files.
		</div>
		{#if selfUpdating}
			<div class="line small hint">
				Self-updating exports hold packweaver and download the pack when the game starts
			</div>
		{/if}
	</section>

	<section>
		<div class="band bar"><span class="n">5</span>Publish to</div>
		<div class="line">
			<label class="check"
				><input
					type="checkbox"
					checked={github}
					disabled={prefs.updates === 'github'}
					onchange={(e) => update((r) => (r.publish.github = e.currentTarget.checked))}
				/>GitHub release</label
			>
			<input
				class="field mono grow"
				value={prefs.publishRepo}
				placeholder={repo ?? 'owner/name'}
				class:prefilled={!!repo}
				spellcheck="false"
				onchange={(e) => setPublishRepo(e.currentTarget.value)}
			/>
			{#if github}{@render visibility(publishAccess, publishBusy, badPublish)}{/if}
			{#if prefs.updates === 'github'}<span class="small hint">required for updates</span>{/if}
			{#if github && access && !access.signedIn}
				<span class="tag changed">not signed in</span>
				<button class="btn small" onclick={() => void openSettingsWindow('accounts')}
					>Sign in…</button
				>
			{/if}
		</div>
		{#if github && publishHint && (prefs.updates !== 'github' || typedPublish !== '')}
			<div class="line small hint">{publishHint}</div>
		{/if}
		{#if elsewhere}
			<div class="line field-error">
				Players update from somewhere else, {repo}. Are you sure this is correct?
			</div>
		{/if}
		{#if selfUpdating}
			<div class="line small hint">Modrinth and CurseForge don't take self-updating packs</div>
		{/if}
		<div class="line" class:muted={selfUpdating}>
			<label class="check"
				><input
					type="checkbox"
					checked={prefs.publish.modrinth && !selfUpdating}
					disabled={selfUpdating}
					onchange={(e) => update((r) => (r.publish.modrinth = e.currentTarget.checked))}
				/>Modrinth</label
			>
			<input
				class="field mono grow"
				value={prefs.modrinthProject}
				placeholder="Project ID"
				disabled={selfUpdating}
				onchange={(e) => update((r) => (r.modrinthProject = e.currentTarget.value.trim()))}
			/>
			{#if !tokens.modrinth && !selfUpdating}
				<button class="btn small" onclick={() => void openSettingsWindow('accounts')}
					>Add token…</button
				>
			{/if}
		</div>
		<div class="line" class:muted={selfUpdating}>
			<label class="check"
				><input
					type="checkbox"
					checked={prefs.publish.curseforge && !selfUpdating}
					disabled={selfUpdating}
					onchange={(e) => update((r) => (r.publish.curseforge = e.currentTarget.checked))}
				/>CurseForge</label
			>
			<input
				class="field mono grow"
				value={prefs.curseforgeProject}
				placeholder="Project ID"
				disabled={selfUpdating}
				onchange={(e) => update((r) => (r.curseforgeProject = e.currentTarget.value.trim()))}
			/>
			{#if !tokens.curseforge && !selfUpdating}
				<button class="btn small" onclick={() => void openSettingsWindow('accounts')}
					>Add token…</button
				>
			{/if}
		</div>
	</section>
</div>

<style>
	.page {
		padding-bottom: 32px;
	}

	section {
		margin-bottom: 8px;
	}

	.n {
		width: 20px;
		height: 20px;
		display: grid;
		place-items: center;
		background: var(--band);
		font: 600 11px var(--font-mono);
	}

	.line {
		display: flex;
		align-items: center;
		gap: 10px;
		min-height: 36px;
		padding: 4px 16px;
	}

	.version {
		width: 160px;
	}

	.muted {
		opacity: 0.42;
	}

	.line.hint {
		min-height: 0;
		padding-top: 0;
		padding-bottom: 8px;
	}

	.label {
		width: 120px;
		flex: none;
	}

	.grow {
		flex: 1;
		max-width: 420px;
	}

	.notes {
		margin: 0 16px;
		width: calc(100% - 32px);
	}
</style>

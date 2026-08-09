<script lang="ts">
	import { onMount } from 'svelte'
	import {
		Package,
		Check,
		ChevronLeft,
		ChevronRight,
		Boxes,
		Server,
		Monitor,
		TriangleAlert,
		UploadCloud,
		ExternalLink,
	} from '@lucide/svelte'
	import Modal from './ui/Modal.svelte'
	import ButtonStyled from './ui/ButtonStyled.svelte'
	import { api, openExternal } from '../api'
	import { store } from '../lib/store.svelte'
	import { GIT_EMPTY_TREE as EMPTY_TREE } from '../util'
	import type { LockedMod, PackEnv } from '../types'
	import { KEYCHAIN } from '../lib/features'

	let { onclose }: { onclose?: () => void } = $props()

	type Format = 'mrpack' | 'curseforge' | 'prism' | 'publish'
	type Platform = 'modrinth' | 'curseforge'

	const GEN: Record<Platform, string> = {
		modrinth: 'https://modrinth.com/settings/pats',
		curseforge: 'https://legacy.curseforge.com/account/api-tokens',
	}
	let tokenInput = $state('')

	async function saveToken() {
		const v = tokenInput.trim()
		if (!v) return
		try {
			await api.secretSet(tokenName(platform), v)
			store.setPref(`secret:${tokenName(platform)}`, true)
			tokenReady = true
			tokenInput = ''
			store.notify('success', 'Token saved')
		} catch (e) {
			store.notify('error', `${e}`)
		}
	}

	let step = $state<1 | 2>(1)
	let env = $state<PackEnv>('common')
	let format = $state<Format | null>(null)
	let selfUpdating = $state(false)

	function setSelfUpdating(v: boolean) {
		selfUpdating = v
		if (v && (format === 'publish' || format === 'curseforge' || !format)) format = 'prism'
	}

	const urlKey = `packUrl:${store.pack?.dir ?? ''}`
	const modeKey = `packUpdateMode:${store.pack?.dir ?? ''}`
	const repoKey = `packRepo:${store.pack?.dir ?? ''}`
	const distKey = `packDistToken:${store.pack?.dir ?? ''}`
	let url = $state(store.getPref(urlKey, ''))
	let selfMode = $state<'commit' | 'release'>(
		store.getPref<string>(modeKey, 'commit') === 'release' ? 'release' : 'commit',
	)
	let repoUrl = $state(store.getPref(repoKey, ''))
	let distToken = $state(store.getPref(distKey, ''))
	let detectedUrl = $state('')

	function withScheme(u: string): string {
		return u && !/^https?:\/\//.test(u) ? `https://${u}` : u
	}
	const packUrlValue = $derived(
		selfMode === 'release'
			? `release+${withScheme(repoUrl.trim().replace(/\/+$/, ''))}`
			: withScheme(url.trim()),
	)
	const distTokenValue = $derived(
		selfMode === 'release' && distToken.trim() ? distToken.trim() : null,
	)

	function repoFromRaw(raw: string): string {
		const m = /raw\.githubusercontent\.com\/([^/]+)\/([^/]+)/.exec(raw)
		return m ? `https://github.com/${m[1]}/${m[2]}` : ''
	}

	async function publishRelease() {
		if (!store.pack) return
		store.setPref(repoKey, repoUrl.trim())
		store.setPref(distKey, distToken.trim())
		const v = versionNumber.trim() || store.pack.manifest.version
		const ok = await store.createGithubRelease(`v${v}`, v, changelog, distTokenValue)
		if (ok) await store.setPackVersion(v)
	}

	let platform = $state<Platform>('modrinth')
	let projectId = $state('')
	let versionNumber = $state(store.pack?.manifest.version ?? '1.0.0')
	let changelog = $state('')
	let linkMods = $state(false)
	let ghRelease = $state(false)
	let lastTag = $state<string | null>(null)
	let tokenReady = $state(false)

	function projectKey(p: Platform) {
		return `pubProject:${store.pack?.dir ?? ''}:${p}`
	}
	function tokenName(p: Platform) {
		return p === 'curseforge' ? 'curseforge_token' : 'modrinth_token'
	}

	function syncPlatform() {
		projectId = store.getPref(projectKey(platform), '')
		tokenReady = store.getPref(`secret:${tokenName(platform)}`, false)
	}

	onMount(async () => {
		syncPlatform()
		if (!store.pack) return
		try {
			const remote = await api.gitPackUrl(store.pack.dir)
			if (remote) {
				detectedUrl = remote
				if (!url) url = remote
				if (!repoUrl) repoUrl = repoFromRaw(remote)
			}
		} catch {
		}
		try {
			lastTag = await api.gitLatestTag(store.pack.dir)
		} catch {
			lastTag = null
		}
		if (!changelog) {
			try {
				const head = await api.changelogHead(store.pack.dir)
				if (head && head.trim()) changelog = head
			} catch {
			}
		}
		if (!changelog) await genChangelog()
	})

	async function genChangelog() {
		if (!store.pack) return
		try {
			changelog = await api.changelogBetween(
				store.pack.dir,
				lastTag ?? EMPTY_TREE,
				'HEAD',
				undefined,
				linkMods,
				true,
			)
		} catch {
		}
	}
	function toggleLinks() {
		linkMods = !linkMods
		genChangelog()
	}

	function setPlatform(p: Platform) {
		platform = p
		syncPlatform()
	}

	const types = $derived.by(() => {
		const list: { key: PackEnv; label: string; icon: typeof Boxes }[] = [
			{ key: 'common', label: 'Everything', icon: Boxes },
		]
		if (store.hasClientOnly) list.push({ key: 'server', label: 'Server pack', icon: Server })
		if (store.hasServerOnly) list.push({ key: 'client', label: 'Client pack', icon: Monitor })
		return list
	})

	function capableOf(m: LockedMod, provider: string): boolean {
		const s = m.sources[provider]
		return !!s && !!s.downloadUrl
	}
	const publishMissing = $derived.by(() => {
		if (format !== 'publish') return 0
		const mods = (store.lockfile?.mods ?? []).filter((m) => !capableOf(m, platform)).length
		return mods + store.unpublished.length
	})
	const overridesCount = $derived.by(() => {
		if (selfUpdating || !format || format === 'publish') return 0
		let n = store.unpublished.length
		if (format === 'curseforge')
			n += (store.lockfile?.mods ?? []).filter((m) => !capableOf(m, 'curseforge')).length
		return n
	})
	const warn = $derived.by(() => {
		if (format === 'publish') {
			const n = publishMissing
			if (!n) return ''
			const f = n === 1 ? 'file' : 'files'
			const are = n === 1 ? 'is' : 'are'
			const they = n === 1 ? 'it' : 'they'
			const them = n === 1 ? 'it' : 'them'
			if (platform === 'modrinth')
				return `${n} ${f} ${are} not hosted on Modrinth. Modrinth allows external content but will require a manual review. Add a Modrinth source to skip that.`
			return `${n} ${f} ${are} not hosted on CurseForge. Add a CurseForge source or remove ${them} before publishing.`
		}
		const n = overridesCount
		if (!n) return ''
		const f = n === 1 ? 'file' : 'files'
		const are = n === 1 ? "isn't" : "aren't"
		const them = n === 1 ? "it'll" : "they'll"
		return `${n} ${f} ${are} from a valid mod host, so ${them} be bundled into the pack as overrides.`
	})

	const title = $derived(step === 1 ? 'Export: what to include' : 'Export: format')

	const canExport = $derived.by(() => {
		if (store.busy || !store.hasLock || !format) return false
		if (selfUpdating && format !== 'publish') {
			if (selfMode === 'release' ? !repoUrl.trim() : !url.trim()) return false
		}
		if (format === 'publish') {
			if (!projectId.trim() || !versionNumber.trim() || !tokenReady) return false
		}
		return true
	})
	const primaryLabel = $derived.by(() => {
		if (format === 'publish')
			return `Publish to ${platform === 'curseforge' ? 'CurseForge' : 'Modrinth'}`
		return 'Choose file & export'
	})

	function saveSelfPrefs() {
		store.setPref(modeKey, selfMode)
		if (selfMode === 'release') {
			store.setPref(repoKey, repoUrl.trim())
			store.setPref(distKey, distToken.trim())
		} else {
			store.setPref(urlKey, url.trim())
		}
	}

	async function runExport() {
		if (!canExport) return
		if (format === 'mrpack') {
			if (selfUpdating) {
				saveSelfPrefs()
				await store.exportMrpackSelfUpdate(packUrlValue, distTokenValue)
			} else {
				await store.exportPack(env)
			}
		} else if (format === 'curseforge') {
			if (selfUpdating) {
				store.setPref(urlKey, url.trim())
				await store.exportCurseforgeSelfUpdate(url.trim())
			} else {
				await store.exportCurseforge(env)
			}
		} else if (format === 'prism') {
			if (selfUpdating) {
				saveSelfPrefs()
				await store.publishPack(packUrlValue, distTokenValue)
			} else {
				await store.exportInstance(env)
			}
		} else if (format === 'publish') {
			store.setPref(projectKey(platform), projectId.trim())
			const ok = await store.publishToPlatform(
				platform,
				projectId.trim(),
				versionNumber.trim(),
				changelog,
				env,
			)
			if (!ok) return
			await store.setPackVersion(versionNumber.trim())
			if (store.pack && changelog.trim()) {
				try {
					await api.changelogSave(
						store.pack.dir,
						`## ${versionNumber.trim()}\n\n${changelog.trim()}`,
					)
				} catch {
				}
			}
			if (ghRelease) {
				await store.createGithubRelease(
					`v${versionNumber.trim()}`,
					versionNumber.trim(),
					changelog,
					store.getPref(distKey, '').trim() || null,
				)
			}
		}
		onclose?.()
	}
</script>

<Modal {title} onclose={() => onclose?.()}>
	<div class="flex gap-md justify-center mb-4xl">
		<span class="w-6 h-1 rounded-max {step >= 1 ? 'bg-brand' : 'bg-divider'}"></span>
		<span class="w-6 h-1 rounded-max {step >= 2 ? 'bg-brand' : 'bg-divider'}"></span>
	</div>

	{#if step === 1}
		<div class="flex flex-col gap-2">
			{#each types as t (t.key)}
				{@const Icon = t.icon}
				<button
					class="flex items-center gap-3 w-full text-left bg-bg-inset border rounded-md px-3.5 py-3 cursor-pointer {env ===
					t.key
						? 'border-brand bg-brand-highlight'
						: 'border-divider hover:border-divider-dark'}"
					onclick={() => (env = t.key)}
				>
					<Icon size={18} class="shrink-0 {env === t.key ? 'text-contrast' : 'text-secondary'}" />
					<div class="flex-1 min-w-0 text-regular font-semibold text-contrast">{t.label}</div>
					{#if env === t.key}
						<Check size={16} class="text-brand shrink-0" />
					{/if}
				</button>
			{/each}
		</div>
		<div class="flex items-center justify-end gap-2.5 mt-4xl">
			<ButtonStyled color="brand" onclick={() => (step = 2)}>
				Continue <ChevronRight size={15} />
			</ButtonStyled>
		</div>
	{:else}
		<label class="flex items-center gap-lg px-2xs pt-2xs pb-3.5 cursor-pointer">
			<input
				type="checkbox"
				class="w-4 h-4 accent-brand cursor-pointer shrink-0"
				checked={selfUpdating}
				onchange={(e) => setSelfUpdating((e.target as HTMLInputElement).checked)}
			/>
			<span class="text-regular font-semibold text-contrast">Self-updating</span>
		</label>

		<div class="flex flex-col gap-2">
			<button
				class="flex items-center gap-3 w-full text-left bg-bg-inset border rounded-md px-3.5 py-3 cursor-pointer {format ===
				'mrpack'
					? 'border-brand bg-brand-highlight'
					: 'border-divider hover:border-divider-dark'}"
				onclick={() => (format = 'mrpack')}
			>
				<Package size={18} class="shrink-0 {format === 'mrpack' ? 'text-contrast' : 'text-secondary'}" />
				<div class="flex-1 min-w-0 text-regular font-semibold text-contrast">Modrinth (.mrpack)</div>
				{#if format === 'mrpack'}
					<Check size={16} class="text-brand shrink-0" />
				{/if}
			</button>

			<button
				class="flex items-center gap-3 w-full text-left bg-bg-inset border rounded-md px-3.5 py-3 cursor-pointer disabled:opacity-45 disabled:cursor-default disabled:hover:border-divider {format ===
				'curseforge'
					? 'border-brand bg-brand-highlight'
					: 'border-divider hover:border-divider-dark'}"
				disabled={selfUpdating}
				onclick={() => (format = 'curseforge')}
			>
				<Package size={18} class="shrink-0 {format === 'curseforge' ? 'text-contrast' : 'text-secondary'}" />
				<div class="flex-1 min-w-0 text-regular font-semibold text-contrast">CurseForge (.zip)</div>
				{#if selfUpdating}
					<span class="shrink-0 text-micro text-secondary">no self-update</span>
				{:else if format === 'curseforge'}
					<Check size={16} class="text-brand shrink-0" />
				{/if}
			</button>

			<button
				class="flex items-center gap-3 w-full text-left bg-bg-inset border rounded-md px-3.5 py-3 cursor-pointer {format ===
				'prism'
					? 'border-brand bg-brand-highlight'
					: 'border-divider hover:border-divider-dark'}"
				onclick={() => (format = 'prism')}
			>
				<Boxes size={18} class="shrink-0 {format === 'prism' ? 'text-contrast' : 'text-secondary'}" />
				<div class="flex-1 min-w-0 text-regular font-semibold text-contrast">
					Prism / MultiMC (.zip)
				</div>
				{#if format === 'prism'}
					<Check size={16} class="text-brand shrink-0" />
				{/if}
			</button>

			<div class="flex flex-col">
				<button
					class="flex items-center gap-3 w-full text-left bg-bg-inset border rounded-md px-3.5 py-3 cursor-pointer disabled:opacity-45 disabled:cursor-default {format ===
					'publish'
						? 'border-brand bg-brand-highlight rounded-b-none border-b-divider'
						: 'border-divider hover:border-divider-dark disabled:hover:border-divider'}"
					disabled={!KEYCHAIN || selfUpdating}
					onclick={() => (format = 'publish')}
				>
					<UploadCloud size={18} class="shrink-0 {format === 'publish' ? 'text-contrast' : 'text-secondary'}" />
					<div class="flex-1 min-w-0 text-regular font-semibold text-contrast">Publish to a platform</div>
					{#if !KEYCHAIN}
						<span
							class="shrink-0 text-micro font-semibold uppercase tracking-[0.04em] text-secondary bg-button-bg border border-divider rounded-max px-md py-2xs"
							>Coming soon</span
						>
					{:else if format === 'publish'}
						<Check size={16} class="text-brand shrink-0" />
					{/if}
				</button>
				{#if KEYCHAIN && format === 'publish'}
					<div class="flex flex-col gap-2 border border-brand border-t-0 rounded-b-md bg-brand-highlight px-3.5 pt-2.5 pb-3">
						<div class="flex gap-sm">
							<button
								class="flex-1 bg-bg-raised border rounded-sm px-2 py-sm text-small font-semibold cursor-pointer {platform ===
								'modrinth'
									? 'border-brand text-contrast'
									: 'border-divider text-secondary'}"
								onclick={() => setPlatform('modrinth')}
							>
								Modrinth
							</button>
							<button
								class="flex-1 bg-bg-raised border rounded-sm px-2 py-sm text-small font-semibold cursor-pointer {platform ===
								'curseforge'
									? 'border-brand text-contrast'
									: 'border-divider text-secondary'}"
								onclick={() => setPlatform('curseforge')}
							>
								CurseForge
							</button>
						</div>
						<div class="flex gap-2">
							<div class="flex-1 min-w-0">
								<label for="exp-project-id" class="block text-tiny text-secondary mb-sm"
									>Project ID</label
								>
								<input
									id="exp-project-id"
									bind:value={projectId}
									class="w-full bg-bg-raised border border-divider text-contrast rounded-sm px-2.5 py-2 text-regular outline-none focus:border-brand"
									spellcheck="false"
									placeholder={platform === 'curseforge' ? '901984' : 'AABBccdd'}
								/>
							</div>
							<div class="flex-[0_0_7rem]">
								<label for="exp-version" class="block text-tiny text-secondary mb-sm"
									>Version</label
								>
								<input
									id="exp-version"
									bind:value={versionNumber}
									class="w-full bg-bg-raised border border-divider text-contrast rounded-sm px-2.5 py-2 text-regular outline-none focus:border-brand"
									spellcheck="false"
									placeholder="1.0.0"
								/>
							</div>
						</div>
						<div class="flex items-center justify-between mb-sm">
							<label for="exp-changelog" class="block text-tiny text-secondary">Changelog</label>
							<div class="flex items-center gap-3">
								<label class="flex items-center gap-sm text-tiny text-secondary cursor-pointer">
									<input
										type="checkbox"
										class="w-[0.85rem] h-[0.85rem] accent-brand cursor-pointer"
										checked={linkMods}
										onchange={toggleLinks}
									/>
									Link mods
								</label>
								<button
									type="button"
									class="bg-transparent border-none text-link text-tiny cursor-pointer hover:underline"
									onclick={genChangelog}>Regenerate from changes</button
								>
							</div>
						</div>
						<textarea
							id="exp-changelog"
							bind:value={changelog}
							class="w-full bg-bg-raised border border-divider text-contrast rounded-sm px-2.5 py-2 text-regular outline-none focus:border-brand resize-y font-mono leading-[1.5]"
							rows="6"
							placeholder="What changed in this version…"
						></textarea>
						<label class="flex items-center gap-md text-tiny text-secondary cursor-pointer">
							<input
								type="checkbox"
								class="w-[0.85rem] h-[0.85rem] accent-brand cursor-pointer shrink-0"
								checked={ghRelease}
								onchange={() => (ghRelease = !ghRelease)}
							/>
							Create a GitHub release
						</label>
						{#if !tokenReady}
							<div class="flex flex-col gap-sm">
								<div class="flex items-center justify-between">
									<span class="text-tiny text-secondary"
										>{platform === 'curseforge' ? 'CurseForge' : 'Modrinth'} token</span
									>
									<button
										type="button"
										class="inline-flex items-center gap-1 bg-transparent border-none text-link text-tiny cursor-pointer hover:underline"
										onclick={() => openExternal(GEN[platform])}
									>
										<ExternalLink size={11} /> Generate one
									</button>
								</div>
								<div class="flex gap-md">
									<input
										bind:value={tokenInput}
										type="password"
										spellcheck="false"
										placeholder="Paste your access token"
										class="flex-1 min-w-0 bg-bg-raised border border-divider text-contrast rounded-sm px-2.5 py-2 text-regular outline-none focus:border-brand"
										onkeydown={(e) => e.key === 'Enter' && saveToken()}
									/>
									<ButtonStyled size="small" color="brand" disabled={!tokenInput.trim()} onclick={saveToken}>
										Save
									</ButtonStyled>
								</div>
							</div>
						{/if}
					</div>
				{/if}
			</div>
		</div>

		{#if selfUpdating && format !== 'publish'}
			<div class="mt-2.5 border border-brand rounded-md bg-brand-highlight px-3.5 pt-2.5 pb-3">
				<div class="flex items-center gap-lg mb-lg">
					<span class="text-tiny text-secondary">Update from</span>
					<div class="flex gap-sm">
						<button
							class="bg-bg-raised border rounded-sm px-2 py-sm text-small font-semibold cursor-pointer {selfMode ===
							'commit'
								? 'border-brand text-contrast'
								: 'border-divider text-secondary'}"
							onclick={() => (selfMode = 'commit')}
						>
							Every commit
						</button>
						<button
							class="bg-bg-raised border rounded-sm px-2 py-sm text-small font-semibold cursor-pointer {selfMode ===
							'release'
								? 'border-brand text-contrast'
								: 'border-divider text-secondary'}"
							onclick={() => (selfMode = 'release')}
						>
							Releases
						</button>
					</div>
				</div>
				{#if selfMode === 'commit'}
					<label for="exp-url" class="block text-tiny text-secondary mb-sm">URL to your pack files</label>
					<!-- svelte-ignore a11y_autofocus -->
					<input
						id="exp-url"
						bind:value={url}
						class="w-full bg-bg-raised border border-divider text-contrast rounded-sm px-2.5 py-2 text-regular outline-none focus:border-brand"
						autofocus
						spellcheck="false"
						placeholder={detectedUrl || 'https://raw.githubusercontent.com/you/pack/main'}
						onkeydown={(e) => e.key === 'Enter' && runExport()}
					/>
					<p class="mt-2 mb-0 text-tiny text-secondary leading-[1.5]">
						Players sync from your committed files on every launch.
					</p>
				{:else}
					<label for="exp-repo" class="block text-tiny text-secondary mb-sm">GitHub repository</label>
					<input
						id="exp-repo"
						bind:value={repoUrl}
						class="w-full bg-bg-raised border border-divider text-contrast rounded-sm px-2.5 py-2 text-regular outline-none focus:border-brand"
						spellcheck="false"
						placeholder="https://github.com/you/your-pack"
					/>
					<label for="exp-dist-token" class="block text-tiny text-secondary mt-2 mb-sm"
						>Access token <em class="not-italic opacity-70">for private repos only</em></label
					>
					<input
						id="exp-dist-token"
						bind:value={distToken}
						type="password"
						class="w-full bg-bg-raised border border-divider text-contrast rounded-sm px-2.5 py-2 text-regular outline-none focus:border-brand"
						spellcheck="false"
						placeholder="github_pat_…"
					/>
					<p class="mt-2 mb-0 text-tiny text-secondary leading-[1.5]">
						Players sync when you publish a GitHub release. The token ships inside the export so
						players can read the private repo, preferably use a token limited to this repository
						with read-only Contents.
					</p>
					<div class="mt-lg flex items-center gap-2.5">
						<ButtonStyled
							size="small"
							type="outlined"
							disabled={store.busy || !repoUrl.trim()}
							onclick={publishRelease}
						>
							Publish release v{versionNumber.trim() || store.pack?.manifest.version}
						</ButtonStyled>
						<span class="text-tiny text-secondary">Uses your generated changelog as the notes.</span>
					</div>
				{/if}
				{#if format === 'mrpack'}
					<p class="mt-2 mb-0 text-tiny text-secondary leading-[1.5]">
						The pack ships packweaver as a mod, so it updates itself on launch in any launcher.
						Mod changes finish when the game closes.
					</p>
				{/if}
			</div>
		{/if}

		{#if warn}
			<div class="flex gap-2 items-start bg-orange/[0.12] border border-orange/[0.35] rounded-md px-3 py-2.5 mt-3.5 text-small text-body leading-[1.45]">
				<TriangleAlert size={15} class="text-orange shrink-0 mt-2xs" />
				<span>{warn}</span>
			</div>
		{/if}

		<div class="flex items-center justify-end gap-2.5 mt-4xl">
			<div class="mr-auto">
				<ButtonStyled type="transparent" onclick={() => (step = 1)}>
					<ChevronLeft size={15} /> Back
				</ButtonStyled>
			</div>
			<ButtonStyled color="brand" disabled={!canExport} onclick={runExport}>
				{primaryLabel}
			</ButtonStyled>
		</div>
	{/if}
</Modal>

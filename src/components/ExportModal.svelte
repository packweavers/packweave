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
	let url = $state(store.getPref(urlKey, ''))
	let detectedUrl = $state('')

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
		if (selfUpdating && format !== 'publish' && !url.trim()) return false
		if (format === 'publish') {
			if (!projectId.trim() || !versionNumber.trim() || !tokenReady) return false
		}
		return true
	})
	const primaryLabel = $derived.by(() => {
		if (format === 'publish')
			return `Publish to ${platform === 'curseforge' ? 'CurseForge' : 'Modrinth'}`
		if (format === 'prism' && selfUpdating) return 'Generate & choose folder'
		return 'Choose file & export'
	})

	async function runExport() {
		if (!canExport) return
		if (format === 'mrpack') {
			if (selfUpdating) {
				store.setPref(urlKey, url.trim())
				await store.exportMrpackSelfUpdate(url.trim())
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
				store.setPref(urlKey, url.trim())
				await store.publishPack(url.trim())
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
				await store.createGithubRelease(`v${versionNumber.trim()}`, versionNumber.trim(), changelog)
			}
		}
		onclose?.()
	}
</script>

<Modal {title} onclose={() => onclose?.()}>
	<div class="flex gap-[0.4rem] justify-center mb-[1.1rem]">
		<span class="w-6 h-1 rounded-max {step >= 1 ? 'bg-brand' : 'bg-divider'}"></span>
		<span class="w-6 h-1 rounded-max {step >= 2 ? 'bg-brand' : 'bg-divider'}"></span>
	</div>

	{#if step === 1}
		<div class="flex flex-col gap-2">
			{#each types as t (t.key)}
				{@const Icon = t.icon}
				<button
					class="flex items-center gap-[0.7rem] w-full text-left bg-bg-inset border rounded-md px-[0.85rem] py-[0.8rem] cursor-pointer {env ===
					t.key
						? 'border-brand bg-brand-highlight'
						: 'border-divider hover:border-divider-dark'}"
					onclick={() => (env = t.key)}
				>
					<Icon size={18} class="shrink-0 {env === t.key ? 'text-contrast' : 'text-secondary'}" />
					<div class="flex-1 min-w-0 text-[0.88rem] font-semibold text-contrast">{t.label}</div>
					{#if env === t.key}
						<Check size={16} class="text-brand shrink-0" />
					{/if}
				</button>
			{/each}
		</div>
		<div class="flex items-center justify-end gap-[0.6rem] mt-[1.1rem]">
			<ButtonStyled color="brand" onclick={() => (step = 2)}>
				Continue <ChevronRight size={15} />
			</ButtonStyled>
		</div>
	{:else}
		<label class="flex items-center gap-[0.55rem] px-[0.1rem] pt-[0.1rem] pb-[0.9rem] cursor-pointer">
			<input
				type="checkbox"
				class="w-4 h-4 accent-brand cursor-pointer shrink-0"
				checked={selfUpdating}
				onchange={(e) => setSelfUpdating((e.target as HTMLInputElement).checked)}
			/>
			<span class="text-[0.88rem] font-semibold text-contrast">Self-updating</span>
		</label>

		<div class="flex flex-col gap-2">
			<button
				class="flex items-center gap-[0.7rem] w-full text-left bg-bg-inset border rounded-md px-[0.85rem] py-[0.8rem] cursor-pointer {format ===
				'mrpack'
					? 'border-brand bg-brand-highlight'
					: 'border-divider hover:border-divider-dark'}"
				onclick={() => (format = 'mrpack')}
			>
				<Package size={18} class="shrink-0 {format === 'mrpack' ? 'text-contrast' : 'text-secondary'}" />
				<div class="flex-1 min-w-0 text-[0.88rem] font-semibold text-contrast">Modrinth (.mrpack)</div>
				{#if format === 'mrpack'}
					<Check size={16} class="text-brand shrink-0" />
				{/if}
			</button>

			<button
				class="flex items-center gap-[0.7rem] w-full text-left bg-bg-inset border rounded-md px-[0.85rem] py-[0.8rem] cursor-pointer disabled:opacity-45 disabled:cursor-default disabled:hover:border-divider {format ===
				'curseforge'
					? 'border-brand bg-brand-highlight'
					: 'border-divider hover:border-divider-dark'}"
				disabled={selfUpdating}
				onclick={() => (format = 'curseforge')}
			>
				<Package size={18} class="shrink-0 {format === 'curseforge' ? 'text-contrast' : 'text-secondary'}" />
				<div class="flex-1 min-w-0 text-[0.88rem] font-semibold text-contrast">CurseForge (.zip)</div>
				{#if selfUpdating}
					<span class="shrink-0 text-[0.66rem] text-secondary">no self-update</span>
				{:else if format === 'curseforge'}
					<Check size={16} class="text-brand shrink-0" />
				{/if}
			</button>

			<button
				class="flex items-center gap-[0.7rem] w-full text-left bg-bg-inset border rounded-md px-[0.85rem] py-[0.8rem] cursor-pointer {format ===
				'prism'
					? 'border-brand bg-brand-highlight'
					: 'border-divider hover:border-divider-dark'}"
				onclick={() => (format = 'prism')}
			>
				<Boxes size={18} class="shrink-0 {format === 'prism' ? 'text-contrast' : 'text-secondary'}" />
				<div class="flex-1 min-w-0 text-[0.88rem] font-semibold text-contrast">
					Prism / MultiMC (.zip)
				</div>
				{#if format === 'prism'}
					<Check size={16} class="text-brand shrink-0" />
				{/if}
			</button>

			<div class="flex flex-col">
				<button
					class="flex items-center gap-[0.7rem] w-full text-left bg-bg-inset border rounded-md px-[0.85rem] py-[0.8rem] cursor-pointer disabled:opacity-45 disabled:cursor-default {format ===
					'publish'
						? 'border-brand bg-brand-highlight rounded-b-none border-b-divider'
						: 'border-divider hover:border-divider-dark disabled:hover:border-divider'}"
					disabled={!KEYCHAIN || selfUpdating}
					onclick={() => (format = 'publish')}
				>
					<UploadCloud size={18} class="shrink-0 {format === 'publish' ? 'text-contrast' : 'text-secondary'}" />
					<div class="flex-1 min-w-0 text-[0.88rem] font-semibold text-contrast">Publish to a platform</div>
					{#if !KEYCHAIN}
						<span
							class="shrink-0 text-[0.64rem] font-semibold uppercase tracking-[0.04em] text-secondary bg-button-bg border border-divider rounded-max px-[0.45rem] py-[0.1rem]"
							>Coming soon</span
						>
					{:else if format === 'publish'}
						<Check size={16} class="text-brand shrink-0" />
					{/if}
				</button>
				{#if KEYCHAIN && format === 'publish'}
					<div class="flex flex-col gap-2 border border-brand border-t-0 rounded-b-md bg-brand-highlight px-[0.85rem] pt-[0.6rem] pb-[0.7rem]">
						<div class="flex gap-[0.3rem]">
							<button
								class="flex-1 bg-bg-raised border rounded-sm px-2 py-[0.35rem] text-[0.8rem] font-semibold cursor-pointer {platform ===
								'modrinth'
									? 'border-brand text-contrast'
									: 'border-divider text-secondary'}"
								onclick={() => setPlatform('modrinth')}
							>
								Modrinth
							</button>
							<button
								class="flex-1 bg-bg-raised border rounded-sm px-2 py-[0.35rem] text-[0.8rem] font-semibold cursor-pointer {platform ===
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
								<label for="exp-project-id" class="block text-[0.72rem] text-secondary mb-[0.3rem]"
									>Project ID</label
								>
								<input
									id="exp-project-id"
									bind:value={projectId}
									class="w-full bg-bg-raised border border-divider text-contrast rounded-sm px-[0.65rem] py-2 text-[0.84rem] outline-none focus:border-brand"
									spellcheck="false"
									placeholder={platform === 'curseforge' ? '901984' : 'AABBccdd'}
								/>
							</div>
							<div class="flex-[0_0_7rem]">
								<label for="exp-version" class="block text-[0.72rem] text-secondary mb-[0.3rem]"
									>Version</label
								>
								<input
									id="exp-version"
									bind:value={versionNumber}
									class="w-full bg-bg-raised border border-divider text-contrast rounded-sm px-[0.65rem] py-2 text-[0.84rem] outline-none focus:border-brand"
									spellcheck="false"
									placeholder="1.0.0"
								/>
							</div>
						</div>
						<div class="flex items-center justify-between mb-[0.3rem]">
							<label for="exp-changelog" class="block text-[0.72rem] text-secondary">Changelog</label>
							<div class="flex items-center gap-[0.7rem]">
								<label class="flex items-center gap-[0.3rem] text-[0.7rem] text-secondary cursor-pointer">
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
									class="bg-transparent border-none text-link text-[0.7rem] cursor-pointer hover:underline"
									onclick={genChangelog}>Regenerate from changes</button
								>
							</div>
						</div>
						<textarea
							id="exp-changelog"
							bind:value={changelog}
							class="w-full bg-bg-raised border border-divider text-contrast rounded-sm px-[0.65rem] py-2 text-[0.84rem] outline-none focus:border-brand resize-y font-mono leading-[1.5]"
							rows="6"
							placeholder="What changed in this version…"
						></textarea>
						<label class="flex items-center gap-[0.4rem] text-[0.74rem] text-secondary cursor-pointer">
							<input
								type="checkbox"
								class="w-[0.85rem] h-[0.85rem] accent-brand cursor-pointer shrink-0"
								checked={ghRelease}
								onchange={() => (ghRelease = !ghRelease)}
							/>
							Create a GitHub release
						</label>
						{#if !tokenReady}
							<div class="flex flex-col gap-[0.3rem]">
								<div class="flex items-center justify-between">
									<span class="text-[0.72rem] text-secondary"
										>{platform === 'curseforge' ? 'CurseForge' : 'Modrinth'} token</span
									>
									<button
										type="button"
										class="inline-flex items-center gap-[0.25rem] bg-transparent border-none text-link text-[0.7rem] cursor-pointer hover:underline"
										onclick={() => openExternal(GEN[platform])}
									>
										<ExternalLink size={11} /> Generate one
									</button>
								</div>
								<div class="flex gap-[0.4rem]">
									<input
										bind:value={tokenInput}
										type="password"
										spellcheck="false"
										placeholder="Paste your access token"
										class="flex-1 min-w-0 bg-bg-raised border border-divider text-contrast rounded-sm px-[0.65rem] py-2 text-[0.84rem] outline-none focus:border-brand"
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
			<div class="mt-[0.6rem] border border-brand rounded-md bg-brand-highlight px-[0.85rem] pt-[0.6rem] pb-[0.7rem]">
				<label for="exp-url" class="block text-[0.72rem] text-secondary mb-[0.3rem]">URL to your pack files</label>
				<!-- svelte-ignore a11y_autofocus -->
				<input
					id="exp-url"
					bind:value={url}
					class="w-full bg-bg-raised border border-divider text-contrast rounded-sm px-[0.65rem] py-2 text-[0.84rem] outline-none focus:border-brand"
					autofocus
					spellcheck="false"
					placeholder={detectedUrl || 'https://raw.githubusercontent.com/you/pack/main'}
					onkeydown={(e) => e.key === 'Enter' && runExport()}
				/>
				<p class="mt-2 mb-0 text-[0.72rem] text-secondary leading-[1.5]">Where your pack files are hosted online.</p>
				{#if format === 'curseforge'}
					<p class="mt-2 mb-0 text-[0.72rem] text-orange leading-[1.5]">
						This format does not self-update on its own, and the CurseForge app has no
						pre-launch hooks. Import the .zip into Prism or MultiMC, then add a pre-launch
						hook:
					</p>
					<code class="block mt-1 bg-bg rounded-[3px] px-[0.4rem] py-[0.25rem] text-[0.72rem] break-all"
						>java -jar "$INST_MC_DIR/packweaver.jar" "$INST_MC_DIR"</code>
				{:else if format !== 'prism' }
					<p class="mt-2 mb-0 text-[0.72rem] text-orange leading-[1.5]">
						This format does not self-update on its own. After importing the pack, add
						packweaver as a pre-launch hook in your launcher:
					</p>
					<div class="mt-[0.4rem] flex flex-col gap-[0.4rem] text-[0.72rem] text-secondary leading-[1.5]">
						<div>
							Modrinth App
							<code class="block mt-[0.15rem] bg-bg rounded-[3px] px-[0.4rem] py-[0.25rem] break-all"
								>java -jar packweaver.jar</code>
						</div>
						<div>
							Prism / MultiMC
							<code class="block mt-[0.15rem] bg-bg rounded-[3px] px-[0.4rem] py-[0.25rem] break-all"
								>java -jar "$INST_MC_DIR/packweaver.jar" "$INST_MC_DIR"</code>
						</div>
					</div>
				{/if}
			</div>
		{/if}

		{#if warn}
			<div class="flex gap-2 items-start bg-orange/[0.12] border border-orange/[0.35] rounded-md px-[0.7rem] py-[0.6rem] mt-[0.9rem] text-[0.78rem] text-body leading-[1.45]">
				<TriangleAlert size={15} class="text-orange shrink-0 mt-[0.1rem]" />
				<span>{warn}</span>
			</div>
		{/if}

		<div class="flex items-center justify-end gap-[0.6rem] mt-[1.1rem]">
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

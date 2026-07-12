<script lang="ts">
	import { onMount } from 'svelte'
	import { Monitor, Moon, Sun, ExternalLink } from '@lucide/svelte'
	import type { Component } from 'svelte'
	import Modal from './ui/Modal.svelte'
	import ButtonStyled from './ui/ButtonStyled.svelte'
	import { api, openExternal } from '../api'
	import { store, type ThemePref } from '../lib/store.svelte'
	import { KEYCHAIN } from '../lib/features'

	const GEN = {
		modrinth: 'https://modrinth.com/settings/pats',
		curseforge: 'https://legacy.curseforge.com/account/api-tokens',
		github: 'https://github.com/settings/tokens/new?description=packweave&scopes=repo',
		gitlab: 'https://gitlab.com/-/user_settings/personal_access_tokens',
	}

	let { onclose }: { onclose?: () => void } = $props()

	const options: { value: ThemePref; label: string; icon: Component }[] = [
		{ value: 'system', label: 'System', icon: Monitor },
		{ value: 'light', label: 'Light', icon: Sun },
		{ value: 'dark', label: 'Dark', icon: Moon },
	]

	type Which = 'modrinth' | 'curseforge' | 'git'
	const KEY: Record<Which, string> = {
		modrinth: 'modrinth_token',
		curseforge: 'curseforge_token',
		git: 'git_token',
	}

	let modrinthToken = $state('')
	let cfToken = $state('')
	let gitToken = $state('')
	let modrinthSaved = $state(false)
	let cfSaved = $state(false)
	let gitSaved = $state(false)

	onMount(() => {
		modrinthSaved = store.getPref(`secret:${KEY.modrinth}`, false)
		cfSaved = store.getPref(`secret:${KEY.curseforge}`, false)
		gitSaved = store.getPref(`secret:${KEY.git}`, false)
	})

	function tokenValue(which: Which): string {
		if (which === 'modrinth') return modrinthToken
		if (which === 'curseforge') return cfToken
		return gitToken
	}

	function setSaved(which: Which, v: boolean) {
		if (which === 'modrinth') modrinthSaved = v
		else if (which === 'curseforge') cfSaved = v
		else gitSaved = v
	}

	function clearValue(which: Which) {
		if (which === 'modrinth') modrinthToken = ''
		else if (which === 'curseforge') cfToken = ''
		else gitToken = ''
	}

	async function saveToken(which: Which) {
		const v = tokenValue(which).trim()
		if (!v) return
		try {
			await api.secretSet(KEY[which], v)
			store.setPref(`secret:${KEY[which]}`, true)
			setSaved(which, true)
			clearValue(which)
			store.notify('success', 'Token saved')
		} catch (e) {
			store.notify('error', `${e}`)
		}
	}

	async function clearToken(which: Which) {
		try {
			await api.secretDelete(KEY[which])
			store.setPref(`secret:${KEY[which]}`, false)
			setSaved(which, false)
			store.notify('success', 'Token removed')
		} catch (e) {
			store.notify('error', `${e}`)
		}
	}
</script>

<Modal title="Settings" onclose={() => onclose?.()}>
	<div class="flex items-center justify-between gap-4 pt-[0.4rem] pb-[0.6rem]">
		<div>
			<div class="text-[0.9rem] font-semibold text-contrast">Appearance</div>
		</div>
		<div class="inline-flex bg-bg-inset border border-divider rounded-md p-[2px] gap-[2px]">
			{#each options as o (o.value)}
				{@const Icon = o.icon}
				<button
					class={`inline-flex items-center gap-[0.3rem] border-none text-[0.78rem] font-[550] px-[0.6rem] py-[0.32rem] rounded-[calc(theme(borderRadius.md)-3px)] cursor-pointer transition-colors ${
						store.theme === o.value
							? 'bg-bg-raised text-contrast shadow-raised'
							: 'bg-transparent text-secondary hover:text-body'
					}`}
					onclick={() => store.setTheme(o.value)}
				>
					<Icon size={14} />
					{o.label}
				</button>
			{/each}
		</div>
	</div>

	<div class="h-px bg-divider mt-[0.4rem] mb-[0.9rem]"></div>

	{#if !KEYCHAIN}
		<div class="flex items-center justify-between gap-4 py-[0.4rem]">
			<div>
				<div class="text-[0.9rem] font-semibold text-contrast">Publishing &amp; private repositories</div>
				<div class="text-[0.76rem] text-secondary mt-[0.15rem] leading-[1.45]">
					Publishing to Modrinth and CurseForge, and access to private Git repositories.
				</div>
			</div>
			<span
				class="shrink-0 text-[0.72rem] font-semibold text-secondary bg-button-bg border border-divider rounded-max px-[0.55rem] py-[0.2rem]"
			>
				Coming soon
			</span>
		</div>
	{:else}
	<div class="text-[0.9rem] font-semibold text-contrast mb-[0.7rem]">Publishing tokens</div>

	<div class="mb-[0.6rem]">
		<div class="flex items-center justify-between mb-[0.3rem]">
			<span class="text-[0.76rem] text-secondary">Modrinth</span>
			<button
				class="inline-flex items-center gap-[0.25rem] bg-transparent border-none text-secondary text-[0.74rem] font-[550] cursor-pointer px-[0.3rem] py-[0.15rem] rounded-sm hover:bg-button-bg hover:text-contrast"
				onclick={() => openExternal(GEN.modrinth)}
			>
				<ExternalLink size={12} /> Generate
			</button>
		</div>
		<div class="flex gap-[0.4rem]">
			<input
				bind:value={modrinthToken}
				type="password"
				spellcheck="false"
				placeholder={modrinthSaved ? '•••••••• stored' : 'Personal access token'}
				class="flex-1 min-w-0 bg-bg-inset border border-divider text-contrast rounded-sm px-[0.6rem] py-[0.45rem] text-[0.82rem] outline-none focus:border-brand"
			/>
			<ButtonStyled
				size="small"
				color="brand"
				disabled={!modrinthToken.trim()}
				onclick={() => saveToken('modrinth')}
			>
				Save
			</ButtonStyled>
			{#if modrinthSaved}
				<ButtonStyled
					size="small"
					color="red"
					type="transparent"
					onclick={() => clearToken('modrinth')}
				>
					Remove
				</ButtonStyled>
			{/if}
		</div>
	</div>

	<div class="mb-[0.6rem]">
		<div class="flex items-center justify-between mb-[0.3rem]">
			<span class="text-[0.76rem] text-secondary">CurseForge</span>
			<button
				class="inline-flex items-center gap-[0.25rem] bg-transparent border-none text-secondary text-[0.74rem] font-[550] cursor-pointer px-[0.3rem] py-[0.15rem] rounded-sm hover:bg-button-bg hover:text-contrast"
				onclick={() => openExternal(GEN.curseforge)}
			>
				<ExternalLink size={12} /> Generate
			</button>
		</div>
		<div class="flex gap-[0.4rem]">
			<input
				bind:value={cfToken}
				type="password"
				spellcheck="false"
				placeholder={cfSaved ? '•••••••• stored' : 'Author upload API token'}
				class="flex-1 min-w-0 bg-bg-inset border border-divider text-contrast rounded-sm px-[0.6rem] py-[0.45rem] text-[0.82rem] outline-none focus:border-brand"
			/>
			<ButtonStyled
				size="small"
				color="brand"
				disabled={!cfToken.trim()}
				onclick={() => saveToken('curseforge')}
			>
				Save
			</ButtonStyled>
			{#if cfSaved}
				<ButtonStyled
					size="small"
					color="red"
					type="transparent"
					onclick={() => clearToken('curseforge')}
				>
					Remove
				</ButtonStyled>
			{/if}
		</div>
	</div>

	<div class="h-px bg-divider mt-[0.4rem] mb-[0.9rem]"></div>

	<div class="text-[0.9rem] font-semibold text-contrast">Git access</div>
	<div class="mb-[0.6rem]">
		<div class="flex items-center justify-between mb-[0.3rem]">
			<span class="text-[0.76rem] text-secondary">Private repositories</span>
			<span class="inline-flex gap-[0.3rem]">
				<button
					class="inline-flex items-center gap-[0.25rem] bg-transparent border-none text-secondary text-[0.74rem] font-[550] cursor-pointer px-[0.3rem] py-[0.15rem] rounded-sm hover:bg-button-bg hover:text-contrast"
					onclick={() => openExternal(GEN.github)}
				>
					<ExternalLink size={12} /> GitHub
				</button>
				<button
					class="inline-flex items-center gap-[0.25rem] bg-transparent border-none text-secondary text-[0.74rem] font-[550] cursor-pointer px-[0.3rem] py-[0.15rem] rounded-sm hover:bg-button-bg hover:text-contrast"
					onclick={() => openExternal(GEN.gitlab)}
				>
					<ExternalLink size={12} /> GitLab
				</button>
			</span>
		</div>
		<div class="flex gap-[0.4rem]">
			<input
				bind:value={gitToken}
				type="password"
				spellcheck="false"
				placeholder={gitSaved ? '•••••••• stored' : 'Personal access token'}
				class="flex-1 min-w-0 bg-bg-inset border border-divider text-contrast rounded-sm px-[0.6rem] py-[0.45rem] text-[0.82rem] outline-none focus:border-brand"
			/>
			<ButtonStyled size="small" color="brand" disabled={!gitToken.trim()} onclick={() => saveToken('git')}>
				Save
			</ButtonStyled>
			{#if gitSaved}
				<ButtonStyled
					size="small"
					color="red"
					type="transparent"
					onclick={() => clearToken('git')}
				>
					Remove
				</ButtonStyled>
			{/if}
		</div>
	</div>
	{/if}

	<div class="h-px bg-divider mt-[0.4rem] mb-[0.9rem]"></div>

	<div>
		<div class="text-[0.88rem] font-semibold text-contrast">packweave</div>
	</div>
</Modal>

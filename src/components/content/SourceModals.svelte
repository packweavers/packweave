<script lang="ts">
	import { ExternalLink } from '@lucide/svelte'
	import Avatar from '../ui/Avatar.svelte'
	import ButtonStyled from '../ui/ButtonStyled.svelte'
	import Modal from '../ui/Modal.svelte'
	import { tooltip } from '../../lib/tooltip'
	import { openCurseforgePage, openModrinthPage } from '../../api'
	import { store } from '../../lib/store.svelte'
	import { providerLabel } from '../../lib/mods'
	import type { ConvertCandidate, LockedMod } from '../../types'

	let convertPending = $state<{
		projectId: string
		oldName: string
		target: string
		projectType: string
		candidate: ConvertCandidate
	} | null>(null)

	let manual = $state<{
		projectId: string
		oldName: string
		target: string
		projectType: string
		query: string
	} | null>(null)

	export async function start(mod: LockedMod, target: string) {
		const candidate = await store.convertSearch(mod.projectId, target)
		if (candidate) {
			convertPending = {
				projectId: mod.projectId,
				oldName: mod.name,
				target,
				projectType: mod.projectType,
				candidate,
			}
		} else {
			manual = {
				projectId: mod.projectId,
				oldName: mod.name,
				target,
				projectType: mod.projectType,
				query: '',
			}
		}
	}

	function enterManual() {
		const p = convertPending
		if (!p) return
		manual = {
			projectId: p.projectId,
			oldName: p.oldName,
			target: p.target,
			projectType: p.projectType,
			query: '',
		}
		convertPending = null
	}

	async function lookupManual() {
		const m = manual
		if (!m || !m.query.trim()) return
		const candidate = await store.convertLookup(m.projectId, m.target, m.query.trim())
		if (candidate) {
			convertPending = {
				projectId: m.projectId,
				oldName: m.oldName,
				target: m.target,
				projectType: m.projectType,
				candidate,
			}
			manual = null
		}
	}

	function openCandidate() {
		const p = convertPending
		if (!p) return
		if (p.target === 'curseforge') openCurseforgePage(p.candidate.slug, p.projectType)
		else openModrinthPage(p.candidate.slug)
	}

	async function confirmAdd() {
		const p = convertPending
		if (!p) return
		convertPending = null
		await store.addAltSource(p.projectId, p.target, p.candidate)
	}
</script>

{#if convertPending}
	<Modal title="Add a source" onclose={() => (convertPending = null)}>
		<p class="text-[0.9rem] text-contrast mb-[0.4rem]">
			Found this on {providerLabel(convertPending.target)}:
		</p>
		<div
			class="flex items-center gap-[0.7rem] p-[0.7rem] border border-divider rounded-md bg-bg-inset mb-4"
		>
			<Avatar src={convertPending.candidate.iconUrl} alt={convertPending.candidate.name} size={44} />
			<div class="flex-1 min-w-0">
				<div class="font-semibold text-contrast whitespace-nowrap overflow-hidden text-ellipsis">
					{convertPending.candidate.name}
				</div>
				<div class="flex items-center gap-[0.35rem] mt-[0.2rem] text-[0.75rem] text-secondary flex-wrap">
					{#if convertPending.candidate.author}
						<span class="inline-flex items-center gap-[0.3rem]">
							{#if convertPending.candidate.authorIconUrl}
								<Avatar
									src={convertPending.candidate.authorIconUrl}
									alt={convertPending.candidate.author}
									size={15}
								/>
							{/if}
							{convertPending.candidate.author}
						</span>
					{/if}
					{#if convertPending.candidate.author && convertPending.candidate.version}
						<span class="opacity-50">·</span>
					{/if}
					{#if convertPending.candidate.version}
						<span class="font-mono">{convertPending.candidate.version}</span>
					{/if}
					<span class="opacity-50">·</span>
					<code class="text-[0.85em] bg-bg-raised px-[0.25rem] rounded-sm"
						>{convertPending.candidate.slug}</code
					>
				</div>
			</div>
			<button
				class="grid place-items-center w-8 h-8 rounded-sm border-none bg-button-bg text-body cursor-pointer shrink-0 hover:bg-button-bg-hover hover:text-contrast"
				use:tooltip={`Open on ${providerLabel(convertPending.target)}`}
				onclick={openCandidate}
			>
				<ExternalLink size={15} />
			</button>
		</div>
		<p class="text-[0.8rem] text-secondary mb-4 leading-[1.5]">
			Add it as a source for “{convertPending.oldName}”?
		</p>
		<div class="flex justify-end items-center gap-[0.6rem]">
			<ButtonStyled type="transparent" disabled={store.busy} onclick={enterManual}>
				<span class="mr-auto">Not this one?</span>
			</ButtonStyled>
			<ButtonStyled type="transparent" disabled={store.busy} onclick={() => (convertPending = null)}>
				Cancel
			</ButtonStyled>
			<ButtonStyled color="brand" disabled={store.busy} onclick={confirmAdd}>Add source</ButtonStyled>
		</div>
	</Modal>
{/if}

{#if manual}
	<Modal title="Find it manually" onclose={() => (manual = null)}>
		<p class="text-[0.8rem] text-secondary mb-4 leading-[1.5]">
			Paste the {providerLabel(manual.target)} page link for “{manual.oldName}”.
		</p>
		<input
			bind:value={manual.query}
			class="w-full box-border bg-bg-inset border border-divider rounded-md text-contrast text-[0.85rem] px-[0.7rem] py-[0.55rem] outline-none mb-[1.1rem] focus:border-brand"
			placeholder={manual.target === 'curseforge'
				? 'curseforge.com/minecraft/mc-mods/…'
				: 'modrinth.com/mod/…'}
			spellcheck="false"
			onkeydown={(e) => {
				if (e.key === 'Enter') lookupManual()
			}}
		/>
		<div class="flex justify-end items-center gap-[0.6rem]">
			<ButtonStyled type="transparent" disabled={store.busy} onclick={() => (manual = null)}>
				Cancel
			</ButtonStyled>
			<ButtonStyled color="brand" disabled={!manual.query.trim() || store.busy} onclick={lookupManual}>
				Find
			</ButtonStyled>
		</div>
	</Modal>
{/if}

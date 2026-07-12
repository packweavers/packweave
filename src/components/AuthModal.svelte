<script lang="ts">
	import { ExternalLink } from '@lucide/svelte'
	import Modal from './ui/Modal.svelte'
	import ButtonStyled from './ui/ButtonStyled.svelte'
	import { openExternal } from '../api'
	import { store } from '../lib/store.svelte'
	import { autofocus } from '../lib/autofocus'

	let token = $state('')

	const provider = $derived(store.authPrompt?.provider ?? 'other')
	const label = $derived(provider === 'github' ? 'GitHub' : provider === 'gitlab' ? 'GitLab' : '')
	const title = $derived(label ? `Connect to ${label}` : 'Git authentication')
	const genUrl = $derived.by(() => {
		if (provider === 'github')
			return 'https://github.com/settings/tokens/new?description=packweave&scopes=repo'
		if (provider === 'gitlab') return 'https://gitlab.com/-/user_settings/personal_access_tokens'
		return ''
	})
	const scopeHint = $derived(
		provider === 'github'
			? 'Tick the “repo” scope, create it, then paste it below.'
			: 'Give it the “write_repository” scope, then paste it below.',
	)

	async function save() {
		if (!token.trim()) return
		const t = token
		token = ''
		await store.saveAuthToken(t)
	}
</script>

<Modal {title} onclose={() => store.dismissAuth()}>
	{#if genUrl}
		<ButtonStyled type="outlined" size="small" onclick={() => openExternal(genUrl)}>
			<ExternalLink size={14} /> Generate a token on {label}
		</ButtonStyled>
		<p class="text-[0.76rem] text-secondary mt-2 mb-[0.9rem] leading-[1.5]">{scopeHint}</p>
	{/if}
	<input
		bind:value={token}
		class="w-full bg-bg-inset border border-divider rounded-md px-3 h-9 text-sm text-contrast focus:border-brand outline-none mb-[1.1rem]"
		type="password"
		placeholder="Paste your access token"
		spellcheck="false"
		use:autofocus
		onkeydown={(e) => e.key === 'Enter' && save()}
	/>
	<div class="flex justify-end gap-[0.6rem]">
		<ButtonStyled type="transparent" disabled={store.busy} onclick={() => store.dismissAuth()}>
			Cancel
		</ButtonStyled>
		<ButtonStyled color="brand" disabled={!token.trim() || store.busy} onclick={save}>
			Save & retry
		</ButtonStyled>
	</div>
</Modal>

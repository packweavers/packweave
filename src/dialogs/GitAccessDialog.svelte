<script lang="ts">
	import { errorText } from '@/lib/tauri/call'
	import { saveToken } from '@/lib/tauri/git'
	import { openExternal } from '@/lib/tauri/windows'
	import Dialog from '@/ui/Dialog.svelte'
	import TextField from '@/ui/TextField.svelte'

	interface Props {
		host: string | null
		close: (result?: boolean) => void
	}

	let { host, close }: Props = $props()

	let token = $state('')
	let error = $state<string | null>(null)
	let saving = $state(false)

	const site = $derived(host ?? 'github.com')
	const forge = $derived(forgeOf(site))

	function forgeOf(site: string): { name: string; url: string } {
		const h = site.toLowerCase()
		if (h === 'github.com') {
			return { name: 'GitHub', url: 'https://github.com/settings/personal-access-tokens/new' }
		}
		if (h.includes('github')) return { name: 'GitHub', url: `https://${site}/settings/tokens` }
		if (h.includes('gitlab')) {
			return { name: 'GitLab', url: `https://${site}/-/user_settings/personal_access_tokens` }
		}
		if (h === 'codeberg.org') {
			return { name: 'Codeberg', url: 'https://codeberg.org/user/settings/applications' }
		}
		if (h.includes('forgejo')) {
			return { name: 'Forgejo', url: `https://${site}/user/settings/applications` }
		}
		if (h.includes('gitea')) {
			return { name: 'Gitea', url: `https://${site}/user/settings/applications` }
		}
		return { name: site, url: `https://${site}` }
	}

	async function save(): Promise<void> {
		if (!token.trim()) return
		saving = true
		error = null
		try {
			await saveToken(token.trim(), host)
			close(true)
		} catch (e) {
			error = errorText(e)
		} finally {
			saving = false
		}
	}
</script>

<Dialog title="{forge.name} access required" onclose={() => close()}>
	<div class="form-field">
		<span class="label">Token for {site}</span>
		<TextField
			bind:value={token}
			type="password"
			mono
			autofocus
			{error}
			onkeydown={(e) => e.key === 'Enter' && void save()}
		/>
	</div>
	<button class="link-button" onclick={() => void openExternal(forge.url)}
		>Create one on {forge.name}</button
	>
	{#snippet footer()}
		<button class="btn" onclick={() => close()}>Cancel</button>
		<button class="btn primary" disabled={!token.trim() || saving} onclick={save}
			>Save and continue</button
		>
	{/snippet}
</Dialog>

<style>
	.link-button {
		align-self: flex-start;
		color: var(--blue);
	}

	.link-button:hover {
		text-decoration: underline;
	}
</style>

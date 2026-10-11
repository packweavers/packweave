<script lang="ts">
	import { onMount } from 'svelte'

	import { toasts } from '@/lib/state/toasts.svelte'
	import { deleteSecret, secretStatus, setSecret } from '@/lib/tauri/app'
	import { errorText } from '@/lib/tauri/call'
	import { deleteToken, saveToken } from '@/lib/tauri/git'
	import { openExternal } from '@/lib/tauri/windows'
	import { pending } from '@/ui/pending'

	interface Account {
		id: string
		name: string
		keys: string[]
		host: string | null
		create: string
	}

	const ACCOUNTS: Account[] = [
		{
			id: 'github',
			name: 'GitHub',
			keys: ['git_token:github.com', 'git_token'],
			host: 'github.com',
			create: 'https://github.com/settings/personal-access-tokens/new',
		},
		{
			id: 'gitlab',
			name: 'GitLab',
			keys: ['git_token:gitlab.com'],
			host: 'gitlab.com',
			create: 'https://gitlab.com/-/user_settings/personal_access_tokens',
		},
		{
			id: 'modrinth',
			name: 'Modrinth',
			keys: ['modrinth_token'],
			host: null,
			create: 'https://modrinth.com/settings/pats',
		},
		{
			id: 'curseforge',
			name: 'CurseForge',
			keys: ['curseforge_token'],
			host: null,
			create: 'https://legacy.curseforge.com/account/api-tokens',
		},
	]

	let connected = $state<Record<string, boolean>>({})
	let editing = $state<string | null>(null)
	let token = $state('')
	let error = $state<string | null>(null)

	onMount(() => {
		void refresh()
	})

	async function refresh(): Promise<void> {
		const flat = ACCOUNTS.flatMap((a) => a.keys)
		const found = await secretStatus(flat).catch((error) => {
			toasts.error(error)
			return flat.map(() => false)
		})
		const status: Record<string, boolean> = {}
		let at = 0
		for (const account of ACCOUNTS) {
			status[account.id] = account.keys.some((_, i) => found[at + i])
			at += account.keys.length
		}
		connected = status
	}

	async function save(account: Account): Promise<void> {
		error = null
		try {
			if (account.host) {
				const user = await saveToken(token.trim(), account.host)
				toasts.show(user ? `Connected as ${user}` : `Saved ${account.name} token`)
			} else {
				await setSecret(account.keys[0], token.trim())
				toasts.show(`Saved ${account.name} token`)
			}
			editing = null
			token = ''
			await refresh()
		} catch (e) {
			error = errorText(e)
		}
	}

	async function remove(account: Account): Promise<void> {
		try {
			if (account.host) await deleteToken(account.host)
			for (const key of account.keys) await deleteSecret(key)
		} catch (error) {
			toasts.error(error)
		} finally {
			await refresh()
		}
	}
</script>

<div class="band bar">Accounts and tokens</div>
{#each ACCOUNTS as account (account.id)}
	<div class="setting">
		<span class="key">{account.name}</span>
		{#if editing === account.id}
			<input
				class="field mono grow"
				type="password"
				bind:value={token}
				placeholder="Token"
				onkeydown={(e) => e.key === 'Enter' && void save(account)}
			/>
			<button class="btn primary" disabled={!token.trim()} onclick={pending(() => save(account))}
				>Save</button
			>
			<button class="btn" onclick={() => ((editing = null), (token = ''), (error = null))}
				>Cancel</button
			>
		{:else if connected[account.id]}
			<span class="tag added">✓ connected</span>
			<span class="spacer"></span>
			<button class="btn" onclick={() => (editing = account.id)}>Replace…</button>
			<button class="btn danger" onclick={pending(() => remove(account))}>Remove</button>
		{:else}
			<span class="spacer"></span>
			<button class="btn quiet" onclick={() => void openExternal(account.create)}>Create one</button
			>
			<button class="btn" onclick={() => (editing = account.id)}>Add token…</button>
		{/if}
	</div>
	{#if editing === account.id && error}<div class="setting">
			<span class="field-error">{error}</span>
		</div>{/if}
{/each}

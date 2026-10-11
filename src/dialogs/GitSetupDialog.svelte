<script lang="ts">
	import { onMount } from 'svelte'

	import { git } from '@/lib/state/git.svelte'
	import { toasts } from '@/lib/state/toasts.svelte'
	import { errorText } from '@/lib/tauri/call'
	import { installGit, recheckSetup, setIdentity } from '@/lib/tauri/git'
	import type { GitSetup } from '@/lib/tauri/types'
	import { openExternal } from '@/lib/tauri/windows'
	import Dialog from '@/ui/Dialog.svelte'
	import TextField from '@/ui/TextField.svelte'

	interface Props {
		close: (result?: boolean) => void
	}

	let { close }: Props = $props()

	let setup = $state<GitSetup | null>(git.setup)
	let name = $state(git.setup?.name ?? '')
	let email = $state(git.setup?.email ?? '')
	let busy = $state(false)
	let error = $state<string | null>(null)

	const installed = $derived(setup?.installed === true)
	const changed = $derived(
		name.trim() !== (setup?.name ?? '') || email.trim() !== (setup?.email ?? ''),
	)

	onMount(() => {
		void recheck()
	})

	async function recheck(): Promise<void> {
		busy = true
		setup = await recheckSetup().catch((error) => {
			toasts.error(error)
			return setup
		})
		git.setup = setup
		name ||= setup?.name ?? ''
		email ||= setup?.email ?? ''
		busy = false
	}

	async function install(): Promise<void> {
		busy = true
		error = null
		try {
			setup = await installGit()
			git.setup = setup
		} catch (e) {
			error = errorText(e)
		} finally {
			busy = false
		}
	}

	async function copyCommand(): Promise<void> {
		if (!setup?.manualCommand) return
		try {
			await navigator.clipboard.writeText(setup.manualCommand)
			toasts.show('Copied')
		} catch (error) {
			toasts.error(error)
		}
	}

	async function save(): Promise<void> {
		if (!changed) {
			close(installed && !!name.trim() && !!email.trim())
			return
		}
		busy = true
		error = null
		try {
			setup = await setIdentity(name.trim(), email.trim())
			git.setup = setup
			close(true)
		} catch (e) {
			error = errorText(e)
		} finally {
			busy = false
		}
	}
</script>

<Dialog title="Set up Git" onclose={() => close()}>
	{#if installed}
		<p><span class="added">✓</span> Git {setup?.version ?? ''}</p>
	{:else if setup}
		<p><span class="removed">✗</span> Git isn't installed</p>
		<div class="row">
			{#if setup.canInstall && setup.installLabel}
				<button class="btn primary" disabled={busy} onclick={install}
					>{busy ? 'Installing…' : setup.installLabel}</button
				>
			{/if}
			<button class="btn" disabled={busy} onclick={recheck}>Check again</button>
			<button class="btn quiet" onclick={() => void openExternal(setup!.downloadUrl)}
				>Download Git</button
			>
		</div>
		{#if setup.manualCommand}
			<div class="command">
				<code class="mono">{setup.manualCommand}</code>
				<button class="btn small" onclick={copyCommand}>Copy</button>
			</div>
		{/if}
	{/if}
	<div class="band">Name and email on commits</div>
	<div class="form-field">
		<span class="label">Name</span>
		<TextField bind:value={name} disabled={!installed} autofocus />
	</div>
	<div class="form-field">
		<span class="label">Email</span>
		<TextField
			bind:value={email}
			disabled={!installed}
			onkeydown={(e) => e.key === 'Enter' && void save()}
		/>
	</div>
	{#if error}<span class="field-error">{error}</span>{/if}
	{#snippet footer()}
		<button class="btn" onclick={() => close()}>Cancel</button>
		<button
			class="btn primary"
			disabled={busy || !installed || !name.trim() || !email.trim()}
			onclick={save}
		>
			{changed ? 'Save' : 'Done'}
		</button>
	{/snippet}
</Dialog>

<style>
	.row {
		display: flex;
		gap: 1px;
	}

	.command {
		display: flex;
		align-items: center;
		gap: 8px;
		padding-left: 10px;
		background: var(--band);
	}

	.command code {
		flex: 1;
		user-select: text;
		-webkit-user-select: text;
		overflow-wrap: anywhere;
	}

	.command .btn {
		align-self: stretch;
		height: auto;
	}
</style>

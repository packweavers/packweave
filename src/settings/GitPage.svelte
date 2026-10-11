<script lang="ts">
	import { onMount } from 'svelte'

	import { app } from '@/lib/state/app.svelte'
	import type { PullStrategy } from '@/lib/state/prefs'
	import { toasts } from '@/lib/state/toasts.svelte'
	import { errorText } from '@/lib/tauri/call'
	import { installGit, recheckSetup, setIdentity } from '@/lib/tauri/git'
	import type { GitSetup } from '@/lib/tauri/types'
	import { openExternal } from '@/lib/tauri/windows'
	import { pending } from '@/ui/pending'
	import Segmented from '@/ui/Segmented.svelte'

	let setup = $state<GitSetup | null>(null)
	let name = $state('')
	let email = $state('')
	let busy = $state(false)

	onMount(() => {
		void check()
	})

	async function check(): Promise<void> {
		busy = true
		try {
			setup = await recheckSetup()
			name = setup.name ?? ''
			email = setup.email ?? ''
		} catch (error) {
			toasts.error(error)
		} finally {
			busy = false
		}
	}

	async function install(): Promise<void> {
		busy = true
		try {
			setup = await installGit()
		} catch (e) {
			toasts.error(e)
		} finally {
			busy = false
		}
	}

	async function saveIdentity(): Promise<void> {
		try {
			setup = await setIdentity(name, email)
			toasts.show('Saved')
		} catch (e) {
			toasts.show(errorText(e), [], 'error')
		}
	}
</script>

<div class="band bar">Git</div>
<div class="setting">
	<span class="key">Git</span>
	{#if setup?.installed}
		<span class="mono small">{setup.path ?? 'git'}</span>
		<span class="small">{setup.version ?? ''}</span>
	{:else if setup}
		<span class="tag removed">✗ not installed</span>
		{#if setup.canInstall && setup.installLabel}
			<button class="btn primary" disabled={busy} onclick={pending(install)}
				>{setup.installLabel}</button
			>
		{:else}
			<button class="btn" onclick={() => void openExternal(setup!.downloadUrl)}>Download Git</button
			>
		{/if}
	{/if}
	<span class="spacer"></span>
	<button class="btn" disabled={busy} onclick={pending(check)}>Check again</button>
</div>
<div class="setting">
	<span class="key">Name on commits</span>
	<input
		class="field grow"
		bind:value={name}
		disabled={!setup?.installed}
		onchange={saveIdentity}
	/>
</div>
<div class="setting">
	<span class="key">Email on commits</span>
	<input
		class="field grow"
		bind:value={email}
		disabled={!setup?.installed}
		onchange={saveIdentity}
	/>
</div>
<div class="setting">
	<span class="key">When pulling</span>
	<Segmented
		options={[
			{ value: 'ff' as PullStrategy, label: 'Fast-forward only' },
			{ value: 'merge' as PullStrategy, label: 'Merge' },
			{ value: 'rebase' as PullStrategy, label: 'Rebase' },
		]}
		value={app.prefs.pullStrategy}
		onchange={(v) => {
			app.prefs.pullStrategy = v
			app.save()
		}}
	/>
</div>

<script lang="ts">
	import { app } from '@/lib/state/app.svelte'
	import { toasts } from '@/lib/state/toasts.svelte'
	import Markdown from '@/ui/Markdown.svelte'

	let checking = $state(false)
	let checked = $state(false)

	async function check(): Promise<void> {
		checking = true
		await app.checkForUpdate(true)
		checking = false
		checked = true
	}

	async function install(): Promise<void> {
		try {
			await app.installUpdate()
		} catch (error) {
			toasts.error(error)
		}
	}
</script>

<div class="band bar">Updates</div>
<div class="setting">
	<span class="key">packweave {app.info?.version ?? ''}</span>
	{#if app.update}
		<span class="tag info">{app.update.version} available</span>
		<span class="spacer"></span>
		<button class="btn primary" onclick={install}>Install and restart</button>
	{:else}
		<span class="small">{checked ? 'Up to date' : ''}</span>
		<span class="spacer"></span>
		<button class="btn" disabled={checking || app.info?.canUpdate === false} onclick={check}
			>{checking ? 'Checking…' : 'Check for updates'}</button
		>
	{/if}
</div>
{#if app.info?.canUpdate === false}
	<div class="setting small">Updated by your package manager</div>
{/if}
{#if app.update?.notes}
	<div class="notes"><Markdown source={app.update.notes} /></div>
{/if}

<style>
	.notes {
		padding: 12px 16px;
	}
</style>

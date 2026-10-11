<script lang="ts">
	import { onMount } from 'svelte'

	import ConfirmDialog from '@/dialogs/ConfirmDialog.svelte'
	import { app } from '@/lib/state/app.svelte'
	import { dialogs } from '@/lib/state/dialogs.svelte'
	import { toasts } from '@/lib/state/toasts.svelte'
	import { cacheSize, clearCache } from '@/lib/tauri/app'
	import { openFolder } from '@/lib/tauri/windows'
	import { bytes } from '@/lib/util/format'
	import { shortenHome } from '@/lib/util/path'
	import { fileManager } from '@/lib/util/platform'

	let size = $state<number | null>(null)
	let clearing = $state(false)

	onMount(() => {
		void cacheSize()
			.then((found) => (size = found))
			.catch((error) => {
				size = null
				toasts.error(error)
			})
	})

	async function clear(): Promise<void> {
		const ok = await dialogs.open(ConfirmDialog, {
			title: 'Clear the cache?',
			message:
				'packweave fetches it again as required. Your packs, settings, sign-ins and sync history stay.',
			confirm: 'Clear cache',
		})
		if (!ok) return
		clearing = true
		try {
			const freed = await clearCache()
			size = 0
			toasts.show(`Cleared ${bytes(freed)}`)
		} catch (error) {
			toasts.error(error)
		} finally {
			clearing = false
		}
	}
</script>

<div class="band bar">Storage</div>
<div class="setting">
	<span class="key">Cache</span>
	<span class="mono small">{size === null ? '…' : bytes(size)}</span>
	<span class="spacer"></span>
	<button
		class="btn"
		disabled={clearing || size === 0}
		aria-busy={clearing}
		onclick={() => void clear()}>{clearing ? 'Clearing…' : 'Clear cache…'}</button
	>
</div>
<div class="setting">
	<span class="key">Data folder</span>
	{#if app.info}<span class="mono small ellipsis">{shortenHome(app.info.dataDir)}</span>{/if}
	<span class="spacer"></span>
	<button class="btn" onclick={() => app.info && void openFolder(app.info.dataDir)}
		>Show in {fileManager}</button
	>
</div>

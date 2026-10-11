<script lang="ts">
	import { onMount } from 'svelte'

	import { app } from '@/lib/state/app.svelte'
	import { toasts } from '@/lib/state/toasts.svelte'
	import { detectInstances } from '@/lib/tauri/instance'
	import type { DetectedInstance } from '@/lib/tauri/types'
	import { loaderName } from '@/lib/util/format'
	import { shortenHome } from '@/lib/util/path'

	let found = $state<DetectedInstance[]>([])

	onMount(() => {
		void detectInstances()
			.then((list) => (found = list))
			.catch((error) => toasts.error(error))
	})

	const launchers = $derived([...new Set(found.map((f) => f.launcher))])

	function toggle(key: 'pushOnSave' | 'gameLogOnLaunch', value: boolean): void {
		app.prefs[key] = value
		app.save()
	}
</script>

<div class="band bar">Launchers and instances</div>
<label class="setting">
	<span class="key">Push saved files</span>
	<input
		type="checkbox"
		class="switch"
		checked={app.prefs.pushOnSave}
		onchange={(e) => toggle('pushOnSave', e.currentTarget.checked)}
	/>
	<span class="small">Saved files are pushed to the linked instance</span>
</label>
<label class="setting">
	<span class="key">Open game log</span>
	<input
		type="checkbox"
		class="switch"
		checked={app.prefs.gameLogOnLaunch}
		onchange={(e) => toggle('gameLogOnLaunch', e.currentTarget.checked)}
	/>
	<span class="small">Game log opens when the linked instance starts</span>
</label>
{#each launchers as launcher (launcher)}
	<div class="band bar">{launcher}</div>
	{#each found.filter((f) => f.launcher === launcher) as instance (instance.gameDir)}
		<div class="setting">
			<span class="key">{instance.name}</span>
			<span class="small"
				>{instance.minecraft ?? '?'} · {loaderName(instance.loader ?? 'vanilla')}</span
			>
			<span class="spacer"></span>
			<span class="mono small ellipsis">{shortenHome(instance.gameDir)}</span>
		</div>
	{/each}
{/each}

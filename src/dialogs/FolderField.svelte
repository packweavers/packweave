<script lang="ts">
	import { documentDir } from '@tauri-apps/api/path'
	import { onMount } from 'svelte'

	import { tasks } from '@/lib/state/tasks.svelte'
	import { pickFolder } from '@/lib/tauri/windows'
	import { shortenHome } from '@/lib/util/path'

	interface Props {
		value: string
		title?: string
	}

	let { value = $bindable(), title = 'Choose a folder' }: Props = $props()

	const KEY = 'lastParentFolder'

	onMount(() => {
		if (value) return
		let saved: string | null = null
		try {
			saved = localStorage.getItem(KEY)
		} catch (error) {
			tasks.fail('Reading the last folder', error)
		}
		if (saved) {
			value = saved
		} else {
			void documentDir()
				.then((dir) => (value ||= dir))
				.catch(tasks.or('Finding the Documents folder', undefined))
		}
	})

	async function change(): Promise<void> {
		const picked = await pickFolder(title)
		if (!picked) return
		value = picked
		try {
			localStorage.setItem(KEY, picked)
		} catch (error) {
			tasks.fail('Remembering the folder', error)
		}
	}
</script>

<div class="folder">
	<span class="field ellipsis mono">{shortenHome(value)}</span>
	<button class="btn" onclick={change}>Change…</button>
</div>

<style>
	.folder {
		display: flex;
		gap: 1px;
	}

	.folder .field {
		flex: 1;
		display: block;
		line-height: var(--row);
	}
</style>

<script lang="ts">
	import { FileText } from '@lucide/svelte'
	import { onMount } from 'svelte'

	import { tabs } from '@/lib/state/tabs.svelte'
	import { tasks } from '@/lib/state/tasks.svelte'
	import { ws } from '@/lib/state/workspace.svelte'
	import { modConfigs } from '@/lib/tauri/files'
	import Icon from '@/ui/Icon.svelte'

	interface Props {
		contentKey: string
	}

	let { contentKey }: Props = $props()

	let configs = $state<string[] | null>(null)

	onMount(() => {
		const dir = ws.dir
		if (!dir) return
		void modConfigs(dir, contentKey)
			.then((found) => (configs = found))
			.catch((error) => {
				configs = []
				tasks.fail('Finding config files', error)
			})
	})
</script>

{#if configs && configs.length > 0}
	<section class="inspector-section">
		<div class="band">Configs</div>
		{#each configs as rel (rel)}
			<button class="item-row" onclick={() => tabs.open('file', rel, { preview: true })}>
				<Icon icon={FileText} size={14} />
				<span class="mono small ellipsis">{rel.replace(/^overrides\//, '')}</span>
			</button>
		{/each}
	</section>
{/if}

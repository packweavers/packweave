<script lang="ts">
	import { DATAPACK_FOLDERS } from '@/lib/content/types'
	import { chooseDatapackFolder } from '@/lib/flows/pack'

	let folder = $state('datapacks')
	let saving = $state(false)

	async function save(): Promise<void> {
		saving = true
		try {
			await chooseDatapackFolder(folder)
		} finally {
			saving = false
		}
	}
</script>

<div class="setup">
	<h2 class="title-doc">Where should data packs go?</h2>
	<p>
		Minecraft only loads data packs per world. Datapack loader mods each require their own folder
		for datapacks to be installed in. Use the folder you need.
	</p>
	<div class="line">
		<input
			class="field mono grow"
			bind:value={folder}
			spellcheck="false"
			onkeydown={(e) => e.key === 'Enter' && void save()}
		/>
		<button
			class="btn primary"
			disabled={saving || !folder.trim()}
			aria-busy={saving}
			onclick={() => void save()}>Use this folder</button
		>
	</div>
	<div class="presets">
		{#each DATAPACK_FOLDERS as preset (preset.folder)}
			<button class="preset" onclick={() => (folder = preset.folder)}>
				<span class="mono">{preset.folder}</span>
				<span class="small hint">{preset.by}</span>
			</button>
		{/each}
	</div>
	<p class="small hint">You can change it later in Pack settings.</p>
</div>

<style>
	.setup {
		max-width: 560px;
		margin: 48px auto;
		padding: 0 24px;
		display: flex;
		flex-direction: column;
		gap: 14px;
	}

	.line {
		display: flex;
		gap: 8px;
	}

	.grow {
		flex: 1;
	}

	.presets {
		display: flex;
		flex-direction: column;
		border: 1px solid var(--line);
	}

	.preset {
		display: flex;
		align-items: center;
		gap: 12px;
		padding: 8px 12px;
		text-align: left;
	}

	.preset + .preset {
		border-top: 1px solid var(--line);
	}

	.preset:hover {
		background: var(--hover);
	}
</style>

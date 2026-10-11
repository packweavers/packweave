<script lang="ts">
	import icon from '@/assets/icon.png'
	import { app } from '@/lib/state/app.svelte'
	import { openExternal, openFolder } from '@/lib/tauri/windows'
	import { shortenHome } from '@/lib/util/path'
</script>

<div class="about">
	<img src={icon} alt="" width="72" height="72" />
	<h1 class="title-doc">packweave</h1>
	<span class="mono">{app.info?.version ?? ''}</span>
	<p>
		A Minecraft modpack IDE with git-native packs, Modrinth and CurseForge, two-way instance sync,
		and one-click publishing.
	</p>
	<div class="links">
		<button
			class="btn"
			onclick={() => void openExternal('https://github.com/packweavers/packweave')}
			>Source code</button
		>
		<button class="btn" onclick={() => app.info && void openFolder(app.info.dataDir)}
			>Data folder</button
		>
	</div>
	{#if app.info}<span class="small mono">{shortenHome(app.info.dataDir)}</span>{/if}
</div>

<style>
	.about {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 10px;
		padding: 48px 24px;
		text-align: center;
	}

	.links {
		display: flex;
		gap: 1px;
		margin-top: 8px;
	}
</style>

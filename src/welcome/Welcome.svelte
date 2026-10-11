<script lang="ts">
	import icon from '@/assets/icon.png'
	import {
		clonePack,
		importPack,
		newPack,
		openPackAt,
		openPackFolder,
		packFromInstance,
	} from '@/lib/flows/pack'
	import { recentMenu } from '@/lib/menus'
	import { app } from '@/lib/state/app.svelte'
	import { keysFor } from '@/lib/state/commands.svelte'
	import { menus } from '@/lib/state/menus.svelte'
	import { tasks } from '@/lib/state/tasks.svelte'
	import { ago, loaderName } from '@/lib/util/format'
	import { shortenHome } from '@/lib/util/path'
	import { pending } from '@/ui/pending'

	const recents = $derived(app.prefs.recents)
	const starts = [
		{
			title: 'From an instance…',
			detail: 'Mods and config from a launcher instance',
			run: packFromInstance,
			command: 'pack.fromInstance',
		},
		{
			title: 'New pack…',
			detail: 'Empty',
			run: newPack,
			command: 'pack.new',
		},
		{
			title: 'Import a pack…',
			detail: '.mrpack, CurseForge .zip, Prism or MultiMC, packwiz',
			run: () => importPack(),
			command: 'pack.import',
		},
		{
			title: 'Clone from Git…',
			detail: 'Any Git URL',
			run: clonePack,
			command: 'pack.clone',
		},
		{
			title: 'Open a folder…',
			detail: 'An existing pack folder',
			run: openPackFolder,
			command: 'pack.open',
		},
	]
</script>

<div class="welcome">
	{#if app.update}
		<div class="banner info">
			packweave {app.update.version} is ready. It installs when you restart.
			<span class="spacer"></span>
			<button class="btn primary" onclick={pending(() => app.installUpdate())}
				>Restart to update</button
			>
			<button class="btn" onclick={() => (app.update = null)}>Later</button>
		</div>
	{/if}
	{#if tasks.current}
		<div class="banner">
			<span class="spinner"></span>
			{tasks.current.label}…
		</div>
	{/if}
	<div class="columns">
		<section class="left">
			<header class="brand">
				<img src={icon} alt="" width="56" height="56" />
				<div>
					<h1 class="title-display">packweave</h1>
					<p>
						A Minecraft modpack IDE with git-native packs, Modrinth and CurseForge, two-way instance
						sync, and one-click publishing.
					</p>
				</div>
			</header>
			<div class="band">Recent packs</div>
			<div class="recents">
				{#each recents as recent (recent.dir)}
					<button
						class="recent"
						onclick={(e) => void openPackAt(recent.dir, e.altKey)}
						oncontextmenu={(e) => menus.at(e, recentMenu(recent.dir))}
					>
						<span class="stack">
							<span class="medium">{recent.name}</span>
							<span class="small ellipsis"
								>Minecraft {recent.minecraft} · {loaderName(recent.loader)} · {shortenHome(
									recent.dir,
								)}</span
							>
						</span>
						<span class="small">{ago(recent.lastOpened)}</span>
					</button>
				{:else}
					<div class="none small">No packs yet</div>
				{/each}
			</div>
		</section>
		<section class="right">
			<div class="band">Start a pack</div>
			{#each starts as start (start.title)}
				{@const keys = start.command ? keysFor(start.command) : null}
				<button class="start" disabled={!!tasks.current} onclick={() => void start.run()}>
					<span class="stack">
						<span class="medium">{start.title}</span>
						<span class="small">{start.detail}</span>
					</span>
					{#if keys}<span class="kbd">{keys}</span>{/if}
				</button>
			{/each}
			<p class="hint small">Drop a .mrpack, .zip or packwiz pack to import</p>
		</section>
	</div>
</div>

<style>
	.welcome {
		flex: 1;
		min-height: 0;
		display: flex;
		flex-direction: column;
		overflow: auto;
	}

	.columns {
		flex: 1;
		display: flex;
		gap: 40px;
		padding: 48px 56px;
		max-width: 1080px;
		width: 100%;
		margin: 0 auto;
	}

	.left {
		flex: 1.4;
		min-width: 0;
		display: flex;
		flex-direction: column;
		gap: 8px;
	}

	.right {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
		gap: 1px;
		padding-top: 96px;
	}

	.brand {
		display: flex;
		align-items: center;
		gap: 16px;
		margin-bottom: 24px;
	}

	.brand p {
		margin-top: 4px;
	}

	.recents {
		display: flex;
		flex-direction: column;
	}

	.recent,
	.start {
		display: flex;
		align-items: center;
		gap: 12px;
		min-height: 52px;
		padding: 8px 12px;
		text-align: left;
		border-bottom: 1px solid var(--line);
	}

	.recent:hover {
		background: var(--band);
	}

	.start {
		background: var(--band);
		border-bottom: 0;
	}

	.start:not(:disabled):hover {
		background: var(--fill);
	}

	.stack {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
		gap: 2px;
	}

	.none {
		padding: 12px;
	}

	.hint {
		padding: 8px 2px;
	}
</style>

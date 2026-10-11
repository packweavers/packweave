<script lang="ts">
	import { ArrowDown, ArrowUp, CircleAlert, TriangleAlert } from '@lucide/svelte'

	import { linkInstance } from '@/lib/flows/instance'
	import { instanceMenu, remoteMenu } from '@/lib/menus'
	import { bisect } from '@/lib/state/bisect.svelte'
	import { runCommand } from '@/lib/state/commands.svelte'
	import { content } from '@/lib/state/content.svelte'
	import { git } from '@/lib/state/git.svelte'
	import { instance } from '@/lib/state/instance.svelte'
	import { menus } from '@/lib/state/menus.svelte'
	import { panels } from '@/lib/state/panels.svelte'
	import { tabs } from '@/lib/state/tabs.svelte'
	import { tasks } from '@/lib/state/tasks.svelte'
	import { ws } from '@/lib/state/workspace.svelte'
	import { duration, gameLabel } from '@/lib/util/format'
	import Icon from '@/ui/Icon.svelte'

	import InstanceDot from './InstanceDot.svelte'

	let now = $state(Date.now())

	$effect(() => {
		if (!instance.running) return
		const timer = setInterval(() => (now = Date.now()), 1000)
		return () => clearInterval(timer)
	})

	const problems = $derived(content.problems.filter((p) => p.severity !== 'info').length)
	const task = $derived(tasks.current)
	const more = $derived(tasks.running.length - 1)
	const pushCount = $derived(instance.toPush.length)
	const pullCount = $derived(instance.toPull.length + instance.both.length)
	const runtime = $derived(instance.running ? duration(now - instance.running.started * 1000) : '')
	const manifest = $derived(ws.manifest)

	function review(): void {
		tabs.open('instance')
	}
</script>

<footer class="status-bar">
	{#if git.isRepo}
		<button class="segment" onclick={(e) => menus.below(e.currentTarget, remoteMenu())}>
			{#if git.remoteName}
				<span>{git.remoteName}</span>
				<Icon icon={ArrowUp} size={12} /><span class="mono">{git.status?.ahead ?? 0}</span>
				<Icon icon={ArrowDown} size={12} /><span class="mono">{git.status?.behind ?? 0}</span>
			{:else}
				<span>No remote</span>
			{/if}
		</button>
	{/if}
	<button class="segment" onclick={() => panels.open('problems')} aria-label="Problems">
		<Icon icon={TriangleAlert} size={12} />
		<span class="mono">{problems}</span>
	</button>
	{#if git.merging}
		<button class="segment" onclick={() => tabs.open('conflicts')}>
			Merging{git.conflicts.length ? ` · ${git.conflicts.length} conflicts` : ''}
		</button>
	{/if}
	{#if task}
		<button class="segment" onclick={() => panels.open('output')}>
			<span class="spinner"></span>
			<span>{task.label}{more > 0 ? ` · ${more} more` : ''}</span>
		</button>
	{:else if tasks.failures > 0}
		<button class="segment failed" onclick={() => panels.open('output')}>
			<Icon icon={CircleAlert} size={12} />
			<span>{tasks.failures} failed</span>
		</button>
	{/if}

	<span class="spacer"></span>

	<div
		class="instance"
		oncontextmenu={(e) => menus.at(e, instanceMenu())}
		role="group"
		aria-label="Instance"
	>
		{#if instance.status === 'none'}
			<span class="name">No instance</span>
			<button class="btn" onclick={() => void linkInstance()}>Link…</button>
		{:else}
			<InstanceDot status={instance.status} />
			<button class="name link" onclick={review}>{instance.name}</button>
			{#if instance.status === 'running'}
				<span>running {runtime}</span>
			{:else if bisect.active}
				<span>finding a broken mod</span>
				<button class="btn" onclick={() => tabs.open('bisect')}>Open</button>
			{:else if instance.status === 'missing'}
				<span>folder missing</span>
				<button class="btn" onclick={() => void linkInstance()}>Fix…</button>
			{:else if instance.status === 'push'}
				<span>{pushCount} to push</span>
				<button
					class="btn primary"
					disabled={instance.applying}
					aria-busy={instance.applying}
					onclick={() => runCommand('instance.push')}>Push</button
				>
				<button class="btn" onclick={review}>Review</button>
			{:else if instance.status === 'pull'}
				<span>{pullCount} to pull</span>
				<button
					class="btn primary"
					disabled={instance.applying}
					aria-busy={instance.applying}
					onclick={() => runCommand('instance.pull')}>Pull</button
				>
				<button class="btn" onclick={review}>Review</button>
			{:else if instance.status === 'both'}
				<span>{pushCount} to push · {pullCount} to pull</span>
				<button class="btn" onclick={review}>Review</button>
			{:else if instance.status === 'in-sync'}
				<span>in sync</span>
			{:else}
				<span class="spinner"></span>
			{/if}
		{/if}
	</div>

	{#if manifest}
		<button class="segment" onclick={() => tabs.open('pack', 'game')}>
			{gameLabel(manifest.minecraft, manifest.loader, manifest.loaderVersion)}
		</button>
	{/if}
</footer>

<style>
	.status-bar {
		flex: none;
		height: var(--row);
		display: flex;
		align-items: stretch;
		background: var(--chrome);
		border-top: 1px solid var(--line);
		font-size: 12px;
		white-space: nowrap;
	}

	.segment {
		display: inline-flex;
		align-items: center;
		gap: 6px;
		padding: 0 9px;
		flex: none;
	}

	.segment:hover {
		background: var(--band);
	}

	.failed {
		color: var(--red);
	}

	.instance {
		display: inline-flex;
		align-items: center;
		gap: 8px;
		padding-left: 10px;
		background: var(--band);
	}

	.instance > .btn {
		align-self: stretch;
		height: auto;
		padding: 0 10px;
		font-size: 11.5px;
	}

	.instance > .btn + .btn {
		margin-left: -7px;
	}

	.instance:not(:has(> .btn:last-child)) {
		padding-right: 10px;
	}

	.name {
		font-weight: 500;
	}

	.link:hover {
		text-decoration: underline;
	}
</style>

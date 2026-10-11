<script lang="ts">
	import { enabledFor, stepsLeft, type BisectMod } from '@/lib/debug/bisect'
	import { linkInstance } from '@/lib/flows/instance'
	import { bisect } from '@/lib/state/bisect.svelte'
	import { content } from '@/lib/state/content.svelte'
	import { instance } from '@/lib/state/instance.svelte'
	import { tabs, type Doc } from '@/lib/state/tabs.svelte'
	import { toasts } from '@/lib/state/toasts.svelte'
	import { plural } from '@/lib/util/format'
	import Empty from '@/ui/Empty.svelte'
	import { pending } from '@/ui/pending'

	interface Props {
		doc: Doc
	}

	let {}: Props = $props()

	let mods = $state<BisectMod[]>([])
	let original = $state<string[]>([])
	let suspects = $state<string[]>([])
	let keep = $state<string[]>([])
	let filter = $state('')
	let loading = $state(false)
	let loaded = $state(false)

	const session = $derived(bisect.session)
	const shown = $derived(
		mods.filter((m) => m.name.toLowerCase().includes(filter.trim().toLowerCase())),
	)
	const libraries = $derived(
		new Set(content.mods.filter((m) => m.dependencyType === 'dependency').map((m) => m.key)),
	)
	const on = $derived(session ? enabledFor(session) : new Set<string>())
	const testingOn = $derived(session ? session.suspects.filter((f) => on.has(f)) : [])
	const testingOff = $derived(session ? session.suspects.filter((f) => !on.has(f)) : [])
	const pulledIn = $derived(
		session && session.testing.length > 0
			? Math.max(0, testingOn.length - session.testing.length)
			: 0,
	)
	const blocked = $derived(bisect.busy || instance.running !== null)

	$effect(() => {
		if (!bisect.session && instance.bound && !loaded && !loading) void load()
	})

	async function load(): Promise<void> {
		loading = true
		const found = await bisect.collect().catch((error) => {
			toasts.error(error)
			return null
		})
		loading = false
		if (!found) return
		loaded = true
		mods = found.mods
		original = found.original
		suspects = found.mods.filter((m) => !m.key || !libraries.has(m.key)).map((m) => m.file)
	}

	function nameOf(file: string): string {
		return session?.mods.find((m) => m.file === file)?.name ?? file
	}

	function toggle(list: string[], file: string, on: boolean): string[] {
		return on ? [...list, file] : list.filter((f) => f !== file)
	}

	function start(): void {
		void bisect.start(mods, original, suspects, keep)
	}

	async function disableFound(): Promise<void> {
		const ids = (session?.found ?? [])
			.map((file) => session?.mods.find((m) => m.file === file)?.key)
			.filter((id): id is string => !!id)
		await bisect.stop()
		if (ids.length > 0) await content.setDisabled(ids, true)
	}
</script>

{#snippet modList(title: string, files: string[])}
	<div class="band bar">{title}<span class="count-text">{files.length}</span></div>
	{#each files as file (file)}
		<div class="trow"><span class="name">{nameOf(file)}</span></div>
	{:else}
		<div class="trow small hint">None</div>
	{/each}
{/snippet}

<div class="bar doc-toolbar">
	<span class="doc-title">Find a broken mod</span>
	{#if instance.bound}<span class="small">in {instance.name}</span>{/if}
	<span class="spacer"></span>
	{#if session}
		<button class="btn" disabled={blocked} onclick={pending(() => bisect.stop())}
			>Stop and restore</button
		>
	{:else if instance.bound && !loading}
		{#if instance.running}<span class="small hint">Close the game first</span>{/if}
		<button class="btn primary" disabled={suspects.length < 2 || blocked} onclick={start}
			>Start</button
		>
	{/if}
</div>

{#if !instance.bound}
	<Empty title="Link an instance first" detail="Mods get turned off and on in that instance">
		<button class="btn primary" onclick={pending(() => linkInstance())}>Link…</button>
	</Empty>
{:else if !session}
	<div class="scroll page">
		<p class="intro">
			Half the suspects will be turned off in {instance.name}. Start the game from your launcher,
			report if the problem persists. This repeats until you report the problem does not appear
			anymore. Your instance is restored after.
		</p>
		{#if loading}
			<div class="trow"><span class="spinner"></span></div>
		{:else}
			<div class="band bar">
				Suspects
				<span class="count-text">{suspects.length}</span>
				<span class="small hint"
					>about {plural(
						Math.ceil(Math.log2(Math.max(suspects.length, 2))),
						'launch',
						'launches',
					)}</span
				>
				<span class="spacer"></span>
				<input class="field filter" bind:value={filter} placeholder="Filter" spellcheck="false" />
				<button class="btn" onclick={() => (suspects = mods.map((m) => m.file))}>All</button>
				<button class="btn" onclick={() => (suspects = [])}>None</button>
			</div>
			{#each shown as mod (mod.file)}
				<div class="trow">
					<input
						type="checkbox"
						checked={suspects.includes(mod.file)}
						onchange={(e) => (suspects = toggle(suspects, mod.file, e.currentTarget.checked))}
					/>
					<span class="name">{mod.name}</span>
					{#if mod.key && libraries.has(mod.key)}<span class="tag">library</span>{/if}
					{#if !mod.key}<span class="tag">not in pack</span>{/if}
					<span class="spacer"></span>
					<label class="keep small">
						<input
							type="checkbox"
							checked={keep.includes(mod.file)}
							onchange={(e) => (keep = toggle(keep, mod.file, e.currentTarget.checked))}
						/>
						Keep on
					</label>
				</div>
			{:else}
				<div class="trow small hint">No mods in {instance.name}</div>
			{/each}
		{/if}
	</div>
{:else if session.checking}
	<div class="scroll page">
		<h2 class="title-doc">Double-check</h2>
		<p class="intro">
			Everything is on except {session.found?.map(nameOf).join(', ')} and mods that require it. Start
			the game from your launcher. Does the problem happen?
		</p>
		<div class="actions">
			<button class="btn" disabled={blocked} onclick={pending(() => bisect.answer(true))}
				>It happened</button
			>
			<button class="btn" disabled={blocked} onclick={pending(() => bisect.answer(false))}
				>It's gone</button
			>
			<button class="btn" disabled={blocked} onclick={pending(() => bisect.back())}>Back</button>
		</div>
	</div>
{:else if session.found}
	<div class="scroll page">
		{#if session.found.length === 0}
			<h2 class="title-doc">Not one of the suspects</h2>
			<p class="intro">
				The problem happened with all of them off. Try again with other mods, or it may not be a
				mod.
			</p>
		{:else if session.found.length === 1}
			<h2 class="title-doc">Found it: {nameOf(session.found[0])}</h2>
			{#if session.checked === true}
				<p class="intro">Confirmed: the problem is gone without it.</p>
			{:else if session.checked === false}
				<p class="intro">
					The problem still happens without it, so it isn't the only cause. It may take two mods
					together.
				</p>
			{:else if !session.sawProblem}
				<p class="intro">
					The problem never came back during testing, so this is a guess. Make sure it happens with
					everything on, then start again.
				</p>
			{:else}
				<p class="intro">Double-check it to be sure.</p>
			{/if}
		{:else}
			<h2 class="title-doc">One of these</h2>
			<p class="intro">They require each other, so they can't be tested apart.</p>
			{@render modList('Suspects', session.found)}
		{/if}
		<div class="actions">
			{#if session.found.length === 1 && session.checked === null}
				<button class="btn primary" disabled={blocked} onclick={pending(() => bisect.check())}
					>Double-check</button
				>
			{/if}
			{#if session.found.length > 0}
				<button class="btn" disabled={blocked} onclick={pending(() => disableFound())}
					>Disable in the pack and restore</button
				>
			{/if}
			{#each session.found as file (file)}
				{@const id = session.mods.find((m) => m.file === file)?.key}
				{#if id}<button class="btn" onclick={() => tabs.open('mod', id)}>Open {nameOf(file)}</button
					>{/if}
			{/each}
			<button class="btn" disabled={blocked} onclick={pending(() => bisect.back())}
				>Back a step</button
			>
		</div>
	</div>
{:else}
	<div class="scroll page">
		<h2 class="title-doc">
			Step {session.history.length + 1}
			<span class="small hint"
				>{plural(session.suspects.length, 'suspect', 'suspects')} left · about {plural(
					stepsLeft(session),
					'step',
					'steps',
				)} more</span
			>
		</h2>
		<p class="intro">
			{plural(testingOn.length, 'suspect is', 'suspects are')} on{pulledIn
				? `, ${pulledIn} because others require them`
				: ''}. Start the game from your launcher and check. Does the problem happen?
		</p>
		<div class="actions">
			<button class="btn" disabled={blocked} onclick={pending(() => bisect.answer(true))}
				>It happened</button
			>
			<button class="btn" disabled={blocked} onclick={pending(() => bisect.answer(false))}
				>It's gone</button
			>
			{#if session.history.length > 0}
				<button class="btn" disabled={blocked} onclick={pending(() => bisect.back())}
					>Back a step</button
				>
			{/if}
			{#if instance.running}<span class="small hint">Answer after the game closes</span>{/if}
		</div>
		{@render modList('On for this test', testingOn)}
		{@render modList('Off for this test', testingOff)}
	</div>
{/if}

<style>
	.page {
		padding-bottom: 32px;
	}

	.page > h2 {
		display: flex;
		align-items: baseline;
		gap: 12px;
		padding: 20px 16px 0;
	}

	.intro {
		max-width: 680px;
		padding: 12px 16px;
		line-height: 1.5;
	}

	.actions {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 8px;
		padding: 12px 16px;
	}

	.filter {
		width: 160px;
	}

	.keep {
		display: flex;
		align-items: center;
		gap: 6px;
	}
</style>

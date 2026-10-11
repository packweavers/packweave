<script lang="ts">
	import { app } from '@/lib/state/app.svelte'
	import { allCommands } from '@/lib/state/commands.svelte'
	import {
		comboOf,
		effectiveKeys,
		formatCombo,
		normalizeCombo,
		presetKeys,
	} from '@/lib/state/keymap'
	import type { KeymapPreset } from '@/lib/state/prefs'
	import { toasts } from '@/lib/state/toasts.svelte'
	import Segmented from '@/ui/Segmented.svelte'

	let filter = $state<'all' | 'changed' | 'conflicts'>('all')
	let recording = $state<string | null>(null)

	const preset = $derived(presetKeys(app.prefs.keymap.preset))
	const keys = $derived(effectiveKeys(app.prefs.keymap.preset, app.prefs.keymap.custom))
	const conflicts = $derived.by(() => {
		const byCombo = new Map<string, string[]>()
		for (const [id, combo] of Object.entries(keys)) {
			const c = normalizeCombo(combo)
			byCombo.set(c, [...(byCombo.get(c) ?? []), id])
		}
		return new Set([...byCombo.values()].filter((ids) => ids.length > 1).flat())
	})
	const commands = $derived(
		allCommands()
			.filter((c) => !/^tab\.goto[2-9]$/.test(c.id))
			.filter(
				(c) =>
					filter === 'all' ||
					(filter === 'changed' ? c.id in app.prefs.keymap.custom : conflicts.has(c.id)),
			),
	)
	const groups = $derived([...new Set(commands.map((c) => c.group))])

	function setCustom(id: string, combo: string | null): void {
		const custom = { ...app.prefs.keymap.custom }
		if (combo === null) {
			delete custom[id]
		} else {
			custom[id] = combo
		}
		app.prefs.keymap = { ...app.prefs.keymap, custom }
		app.save()
	}

	function onkeydown(event: KeyboardEvent): void {
		if (!recording) return
		event.preventDefault()
		event.stopPropagation()
		if (event.key === 'Escape') {
			recording = null
			return
		}
		if (event.key === 'Backspace' && !event.metaKey && !event.ctrlKey && !event.altKey) {
			setCustom(recording, '')
			recording = null
			return
		}
		const combo = comboOf(event)
		if (!combo || ['Meta', 'Control', 'Alt', 'Shift'].includes(event.key)) return
		setCustom(recording, preset[recording] === combo ? null : combo)
		recording = null
	}

	async function exportKeys(): Promise<void> {
		try {
			await navigator.clipboard.writeText(JSON.stringify(app.prefs.keymap, null, 2))
			toasts.show('Copied your keymap')
		} catch (error) {
			toasts.error(error)
		}
	}

	async function importKeys(): Promise<void> {
		try {
			const parsed = JSON.parse(await navigator.clipboard.readText()) as {
				preset?: KeymapPreset
				custom?: Record<string, string>
			}
			app.prefs.keymap = {
				preset: parsed.preset ?? app.prefs.keymap.preset,
				custom: parsed.custom ?? {},
			}
			app.save()
			toasts.show('Imported a keymap from the clipboard')
		} catch {
			toasts.show("The clipboard doesn't hold a keymap", [], 'error')
		}
	}
</script>

<svelte:window onkeydowncapture={onkeydown} />

<div class="band bar">
	Keyboard
	<span class="spacer"></span>
	<button class="btn" onclick={importKeys}>Import</button>
	<button class="btn" onclick={exportKeys}>Export</button>
	<button
		class="btn"
		disabled={Object.keys(app.prefs.keymap.custom).length === 0}
		onclick={() => {
			app.prefs.keymap = { ...app.prefs.keymap, custom: {} }
			app.save()
		}}
	>
		Reset
	</button>
</div>
<div class="setting">
	<span class="key">Preset</span>
	<Segmented
		options={[
			{ value: 'packweave' as KeymapPreset, label: 'packweave' },
			{ value: 'vscode' as KeymapPreset, label: 'VS Code' },
			{ value: 'jetbrains' as KeymapPreset, label: 'JetBrains' },
		]}
		value={app.prefs.keymap.preset}
		onchange={(preset) => {
			app.prefs.keymap = { ...app.prefs.keymap, preset }
			app.save()
		}}
	/>
	<span class="spacer"></span>
	<Segmented
		small
		options={[
			{ value: 'all', label: 'All' },
			{ value: 'changed', label: 'Changed' },
			{ value: 'conflicts', label: 'Conflicts' },
		]}
		bind:value={filter}
	/>
</div>
{#each groups as group (group)}
	<div class="band bar">{group}</div>
	{#each commands.filter((c) => c.group === group) as command (command.id)}
		{@const combo = keys[command.id]}
		<div class="setting">
			<span class="key wide">{command.title}</span>
			<button
				class="kbd shortcut"
				class:recording={recording === command.id}
				onclick={() => (recording = command.id)}
			>
				{recording === command.id ? 'Press keys…' : combo ? formatCombo(combo) : '—'}
			</button>
			{#if conflicts.has(command.id)}<span class="tag removed">conflict</span>{/if}
			<span class="spacer"></span>
			{#if command.id in app.prefs.keymap.custom}
				<span class="small">changed</span>
				<button class="btn small" onclick={() => setCustom(command.id, null)}>Reset</button>
			{:else}
				<span class="small">preset</span>
			{/if}
		</div>
	{/each}
{/each}

<style>
	.wide {
		flex-basis: 260px;
	}

	.shortcut {
		min-width: 80px;
		height: 24px;
		font-size: 11.5px;
	}

	.shortcut.recording {
		outline: 2px solid var(--focus);
	}
</style>

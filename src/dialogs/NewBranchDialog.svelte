<script lang="ts">
	import { untrack } from 'svelte'

	import type { NewBranchInstance } from '@/lib/state/prefs'
	import Dialog from '@/ui/Dialog.svelte'
	import Segmented from '@/ui/Segmented.svelte'
	import Select from '@/ui/Select.svelte'
	import TextField from '@/ui/TextField.svelte'

	interface Result {
		name: string
		start: string
		instance: 'copy' | 'move' | 'none'
		where: 'window' | 'here'
	}

	interface Props {
		start: string
		branches: string[]
		remotes: string[]
		instanceName: string | null
		canCopy: boolean
		defaultInstance: NewBranchInstance
		suggestion: string
		changeLabel: string | null
		close: (result?: Result) => void
	}

	let {
		start,
		branches,
		remotes,
		instanceName,
		canCopy,
		defaultInstance,
		suggestion,
		changeLabel,
		close,
	}: Props = $props()

	function suggest(text: string): string {
		const slug = text
			.toLowerCase()
			.replace(/^(add|update|try)\s+/, '')
			.replace(/[^a-z0-9]+/g, '-')
			.replace(/^-|-$/g, '')
			.slice(0, 32)
		return slug ? `test/${slug}` : ''
	}

	let name = $state(untrack(() => suggest(suggestion)))
	let from = $state(untrack(() => start))
	let instance = $state<'copy' | 'move' | 'none'>(
		untrack(() => (defaultInstance === 'copy' && !canCopy ? 'none' : defaultInstance)),
	)
	let where = $state<'window' | 'here'>('window')

	const invalid = $derived(
		name.trim().length > 0 &&
			(/[\s~^:?*[\\]|\.\.|@\{|\/$|^\/|\.lock$/.test(name.trim()) || branches.includes(name.trim())),
	)

	function submit(): void {
		if (!name.trim() || invalid) return
		close({ name: name.trim(), start: from, instance, where: changeLabel ? 'window' : where })
	}
</script>

<Dialog title="New branch" onclose={() => close()}>
	<div class="form-field">
		<span class="label">Name</span>
		<TextField
			bind:value={name}
			placeholder="test/iris"
			mono
			autofocus
			error={invalid
				? branches.includes(name.trim())
					? 'That branch exists already.'
					: "Branch names can't have spaces or ~ ^ : ? * [ \\."
				: null}
			onkeydown={(e) => e.key === 'Enter' && submit()}
		/>
	</div>
	<div class="form-field">
		<span class="label">Start from</span>
		<Select
			options={[...branches, ...remotes].map((b) => ({ value: b, label: b }))}
			bind:value={from}
			mono
		/>
	</div>
	{#if instanceName}
		<div class="form-field">
			<span class="label">Instance</span>
			<Segmented
				options={[
					...(canCopy ? [{ value: 'copy' as const, label: `Copy ${instanceName}` }] : []),
					{ value: 'move' as const, label: `Move ${instanceName}` },
					{ value: 'none' as const, label: 'None' },
				]}
				bind:value={instance}
			/>
		</div>
	{/if}
	{#if changeLabel}
		<p class="small">{changeLabel}</p>
	{:else}
		<div class="form-field">
			<span class="label">Open it in</span>
			<Segmented
				options={[
					{ value: 'window', label: 'A new window' },
					{ value: 'here', label: 'This window' },
				]}
				bind:value={where}
			/>
		</div>
	{/if}
	{#snippet footer()}
		<button class="btn" onclick={() => close()}>Cancel</button>
		<button class="btn primary" disabled={!name.trim() || invalid} onclick={submit}
			>Create branch</button
		>
	{/snippet}
</Dialog>

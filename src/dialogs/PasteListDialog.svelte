<script lang="ts">
	import { DATAPACK_FOLDERS } from '@/lib/content/types'
	import { app } from '@/lib/state/app.svelte'
	import { ws } from '@/lib/state/workspace.svelte'
	import { errorText } from '@/lib/tauri/call'
	import { bulkLookup } from '@/lib/tauri/content'
	import type { AddItem, BulkCandidate, BulkLookup, ProviderId } from '@/lib/tauri/types'
	import { typeName } from '@/lib/util/format'
	import Dialog from '@/ui/Dialog.svelte'
	import ModIcon from '@/ui/ModIcon.svelte'
	import Segmented from '@/ui/Segmented.svelte'
	import Select from '@/ui/Select.svelte'

	interface Result {
		items: AddItem[]
		datapacks: string | null
	}

	interface Props {
		close: (result?: Result) => void
	}

	type Entry =
		| { kind: 'found'; candidate: BulkCandidate }
		| { kind: 'in-pack'; candidate: BulkCandidate }
		| { kind: 'missing'; raw: string }

	let { close }: Props = $props()

	let text = $state('')
	let provider = $state<ProviderId>('modrinth')
	let result = $state<BulkLookup | null>(null)
	let skipped = $state<string[]>([])
	let busy = $state(false)
	let error = $state<string | null>(null)
	let datapackFolder = $state(DATAPACK_FOLDERS[0].folder)

	const rows = $derived.by(() => {
		if (!result) return []
		const byLine = new Map<number, Entry[]>()
		const add = (line: number, entry: Entry) =>
			byLine.set(line, [...(byLine.get(line) ?? []), entry])
		for (const candidate of result.found) add(candidate.line, { kind: 'found', candidate })
		for (const candidate of result.inPack) add(candidate.line, { kind: 'in-pack', candidate })
		for (const failed of result.failed) add(failed.line, { kind: 'missing', raw: failed.raw })
		return text
			.split('\n')
			.map((line, index) => ({ index, line, entries: byLine.get(index) ?? [] }))
			.filter((row) => row.line.trim() !== '' || row.entries.length > 0)
	})
	const unique = $derived.by(() => {
		const seen = new Map<string, BulkCandidate>()
		for (const candidate of result?.found ?? []) {
			if (!seen.has(candidate.projectId)) seen.set(candidate.projectId, candidate)
		}
		return [...seen.values()]
	})
	const chosen = $derived(unique.filter((c) => !skipped.includes(c.projectId)))
	const missing = $derived(result?.failed.length ?? 0)
	const needsFolder = $derived(
		!ws.manifest?.datapacks && chosen.some((c) => c.projectType === 'datapack'),
	)

	async function lookUp(): Promise<void> {
		if (!ws.dir || !text.trim()) return
		busy = true
		error = null
		try {
			result = await bulkLookup(ws.dir, provider, text)
			skipped = []
		} catch (e) {
			error = errorText(e)
		} finally {
			busy = false
		}
	}

	function toggle(projectId: string, on: boolean): void {
		skipped = on ? skipped.filter((id) => id !== projectId) : [...skipped, projectId]
	}

	function submit(): void {
		close({
			items: chosen.map((c) => ({
				provider: c.provider,
				projectId: c.projectId,
				slug: c.slug,
				name: c.name,
				projectType: c.projectType,
			})),
			datapacks: needsFolder ? datapackFolder : null,
		})
	}
</script>

<Dialog title="Paste a list" width={720} onclose={() => close()}>
	{#if !result}
		<textarea
			class="field-area mono"
			rows="10"
			placeholder={'sodium\nJust Enough Items\nhttps://modrinth.com/shader/complementary-reimagined\nfresh-animations, ferrite-core\nsodium-fabric-0.5.8+mc1.20.1.jar'}
			bind:value={text}
			spellcheck="false"></textarea>
		<p class="small hint">
			Names, slugs, links or file names. One per line, or separated by commas.
		</p>
		{#if app.providerReady('curseforge')}
			<Segmented
				small
				options={[
					{ value: 'modrinth', label: 'Modrinth' },
					{ value: 'curseforge', label: 'CurseForge' },
				]}
				bind:value={provider}
			/>
		{/if}
	{:else}
		<p class="small">
			{unique.length} found{missing > 0 ? ` · ${missing} not found` : ''}{result.inPack.length > 0
				? ` · ${result.inPack.length} already in the pack`
				: ''}
		</p>
		<div class="lines">
			{#each rows as row (row.index)}
				{#if row.entries.length === 0}
					<div class="line skipped">
						<span class="raw mono">{row.line.trim()}</span>
						<span class="spacer"></span>
						<span class="small">skipped</span>
					</div>
				{/if}
				{#each row.entries as entry, i (i)}
					{#if entry.kind === 'found'}
						<label class="line found">
							<input
								type="checkbox"
								checked={!skipped.includes(entry.candidate.projectId)}
								onchange={(e) => toggle(entry.candidate.projectId, e.currentTarget.checked)}
							/>
							<span class="raw mono">{entry.candidate.raw}</span>
							<span class="arrow">→</span>
							<ModIcon url={entry.candidate.iconUrl} size={16} />
							<span class="name">{entry.candidate.name}</span>
							{#if entry.candidate.projectType !== 'mod'}
								<span class="tag">{typeName(entry.candidate.projectType).toLowerCase()}</span>
							{/if}
							{#if entry.candidate.guess}<span class="tag changed">closest match</span>{/if}
						</label>
					{:else if entry.kind === 'in-pack'}
						<div class="line in-pack">
							<span class="raw mono">{entry.candidate.raw}</span>
							<span class="arrow">→</span>
							<span class="name">{entry.candidate.name}</span>
							<span class="tag">in pack</span>
						</div>
					{:else}
						<div class="line missing">
							<span class="raw mono">{entry.raw}</span>
							<span class="spacer"></span>
							<span class="tag removed">not found</span>
						</div>
					{/if}
				{/each}
			{/each}
		</div>
		{#if needsFolder}
			<div class="form-field folder">
				<span class="label">Data pack folder</span>
				<Select
					options={DATAPACK_FOLDERS.map((p) => ({
						value: p.folder,
						label: `${p.folder} (${p.by})`,
					}))}
					bind:value={datapackFolder}
				/>
			</div>
		{/if}
	{/if}
	{#if error}<span class="field-error">{error}</span>{/if}
	{#snippet footer()}
		{#if result}
			<button class="btn" onclick={() => (result = null)}>Edit list</button>
		{:else}
			<button class="btn primary" disabled={busy || !text.trim()} onclick={lookUp}
				>{busy ? 'Searching…' : 'Search'}</button
			>
		{/if}
		<span class="spacer"></span>
		<button class="btn" onclick={() => close()}>Cancel</button>
		{#if result}
			<button class="btn primary" disabled={chosen.length === 0} onclick={submit}
				>Add {chosen.length}</button
			>
		{/if}
	{/snippet}
</Dialog>

<style>
	.hint {
		margin: 6px 0;
	}

	.folder {
		margin-top: 10px;
	}

	.lines {
		max-height: 420px;
		overflow: auto;
		border: 1px solid var(--line);
		margin-top: 8px;
	}

	.line {
		display: flex;
		align-items: center;
		gap: 8px;
		min-height: 30px;
		padding: 2px 10px;
		font-size: 12.5px;
	}

	.line + .line {
		border-top: 1px solid var(--line);
	}

	.raw {
		max-width: 45%;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.found .raw {
		color: var(--green);
	}

	.missing .raw {
		color: var(--red);
	}

	.skipped,
	.in-pack .raw {
		color: var(--grey);
	}

	.arrow {
		color: var(--grey);
	}

	.name {
		flex: 1;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		font-weight: 500;
	}
</style>

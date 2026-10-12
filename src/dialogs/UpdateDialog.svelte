<script lang="ts">
	import { ArrowUp, ChevronDown, ChevronRight } from '@lucide/svelte'
	import { onMount, untrack } from 'svelte'

	import UpdateNotes from '@/inspectors/UpdateNotes.svelte'
	import { contentChange } from '@/lib/content/changes'
	import { changeMark } from '@/lib/git/changes'
	import { content } from '@/lib/state/content.svelte'
	import { tasks } from '@/lib/state/tasks.svelte'
	import { enrich } from '@/lib/tauri/catalog'
	import type { ImpactReport, ModMeta, VersionPin } from '@/lib/tauri/types'
	import { plural } from '@/lib/util/format'
	import Author from '@/ui/Author.svelte'
	import Dialog from '@/ui/Dialog.svelte'
	import Icon from '@/ui/Icon.svelte'
	import ModIcon from '@/ui/ModIcon.svelte'

	interface Props {
		pins: VersionPin[]
		impact: ImpactReport | null
		preview: (pins: VersionPin[]) => Promise<ImpactReport | null>
		close: (result?: { pins: VersionPin[] }) => void
	}

	let { pins, impact, preview, close }: Props = $props()

	let extraMeta = $state<Record<string, ModMeta>>({})
	let open = $state<Record<string, boolean>>({})
	let picked = $state<Record<string, boolean>>(
		untrack(() => Object.fromEntries(pins.map((pin) => [pin.key, true]))),
	)
	let current = $state<ImpactReport | null>(untrack(() => impact))
	let request = 0

	const chosen = $derived(pins.filter((pin) => picked[pin.key]))
	const all = $derived(chosen.length === pins.length)

	$effect(() => {
		const subset = chosen
		const ticket = ++request
		if (subset.length === pins.length) {
			current = untrack(() => impact)
			return
		}
		if (subset.length === 0) {
			current = null
			return
		}
		const timer = setTimeout(() => {
			void preview(subset)
				.then((report) => {
					if (ticket === request) current = report
				})
				.catch(tasks.or('Checking what else changes', undefined))
		}, 250)
		return () => clearTimeout(timer)
	})

	function pickAll(on: boolean): void {
		picked = Object.fromEntries(pins.map((pin) => [pin.key, on]))
	}

	const rows = $derived(
		pins.map((pin) => {
			const mod = content.find(pin.key)
			const update = content.updates[pin.key]
			const exact = update?.versionId === pin.version
			const planned = current?.changes.find((c) => c.key === pin.key)?.toVersion
			return {
				id: pin.key,
				projectId: mod?.projectId ?? pin.key,
				version: pin.version,
				name: mod?.name ?? pin.key,
				from: mod?.sources[mod.preferred]?.versionNumber ?? '?',
				to: planned ?? (exact ? update.versionNumber : '?'),
				channel: exact ? update.versionType : 'release',
				meta: mod ? content.meta[mod.projectId] : undefined,
			}
		}),
	)
	const asked = $derived(new Set(chosen.map((pin) => pin.key)))
	const extra = $derived((current?.changes ?? []).filter((c) => !asked.has(c.key)))
	const title = $derived(
		pins.length === 1 ? `Update ${rows[0].name}` : `Update ${plural(pins.length, 'mod', 'mods')}`,
	)

	onMount(() => {
		const missing = extra
			.filter((c) => !content.meta[c.projectId])
			.filter((c) => c.provider === 'modrinth' || c.provider === 'curseforge')
			.map((c) => ({ id: c.projectId, provider: c.provider }))
		if (missing.length === 0) return
		void enrich(missing)
			.then((found) => {
				extraMeta = Object.fromEntries(found.map((m) => [m.projectId, m]))
			})
			.catch(tasks.or('Loading icons and authors', undefined))
	})

	function metaOf(projectId: string): ModMeta | undefined {
		return content.meta[projectId] ?? extraMeta[projectId]
	}

	function toggle(row: (typeof rows)[number]): void {
		open[row.id] = !open[row.id]
	}
</script>

<Dialog {title} width={720} onclose={() => close()}>
	<div class="list">
		{#if pins.length > 1}
			<label class="item all">
				<input
					type="checkbox"
					checked={all}
					indeterminate={chosen.length > 0 && !all}
					onchange={(e) => pickAll(e.currentTarget.checked)}
				/>
				<span class="name">Select all</span>
				<span class="spacer"></span>
				<span class="small">{chosen.length} of {pins.length}</span>
			</label>
		{/if}
		{#each rows as row (row.id)}
			<div class="item" class:off={!picked[row.id]}>
				<div class="head">
					{#if pins.length > 1}
						<input
							type="checkbox"
							aria-label="Update {row.name}"
							checked={picked[row.id]}
							onchange={(e) => (picked[row.id] = e.currentTarget.checked)}
						/>
					{/if}
					<button class="toggle" onclick={() => toggle(row)} aria-expanded={!!open[row.id]}>
						<ModIcon url={row.meta?.iconUrl} size={32} />
						<span class="text">
							<span class="name">{row.name}</span>
							{#if row.meta?.author}<span class="small sub"
									><Author name={row.meta.author} icon={row.meta.authorIconUrl} /></span
								>{/if}
						</span>
						<span class="versions mono">
							<span class="version" title={row.from}>{row.from}</span>
							<span class="version update" title={row.to}
								><Icon icon={ArrowUp} size={12} />{row.to}</span
							>
						</span>
						{#if row.channel !== 'release'}<span class="tag changed">{row.channel}</span>{/if}
						<Icon icon={open[row.id] ? ChevronDown : ChevronRight} size={14} />
					</button>
				</div>
				{#if open[row.id]}
					<div class="notes">
						<UpdateNotes projectId={row.projectId} version={row.version} expandAll />
					</div>
				{/if}
			</div>
		{/each}
	</div>

	{#if extra.length > 0}
		<div class="band">Also changes</div>
		<div class="list">
			{#each extra as change (change.projectId)}
				{@const word = contentChange(change.kind)}
				{@const meta = metaOf(change.projectId)}
				<div class="item">
					<div class="head static">
						<ModIcon url={meta?.iconUrl} size={32} />
						<span class="text">
							<span class="name">{change.name}</span>
							<span class="small sub"
								>{#if meta?.author}<Author name={meta.author} icon={meta.authorIconUrl} />{/if}<span
									>{change.dependency ? 'dependency' : 'in your pack'}</span
								></span
							>
						</span>
						<span class="versions mono">
							{#if change.fromVersion}
								<span class="version" title={change.fromVersion}>{change.fromVersion}</span>
							{/if}
							{#if change.toVersion}
								<span class="version" class:update={!!change.fromVersion} title={change.toVersion}
									>{#if change.fromVersion}<Icon
											icon={ArrowUp}
											size={12}
										/>{/if}{change.toVersion}</span
								>
							{/if}
						</span>
						<span class="mark-text {word}">{changeMark(word)}</span>
					</div>
				</div>
			{/each}
		</div>
	{/if}

	{#if current && (current.fixed.length > 0 || current.problems.length > 0)}
		<div class="band">Problems</div>
		<div class="problems">
			{#each current.fixed as fixed (fixed)}
				<p><span class="tag added">✓ fixed</span> {fixed}</p>
			{/each}
			{#each current.problems as problem (problem)}
				<p><span class="tag removed">✗ new</span> {problem}</p>
			{/each}
		</div>
	{/if}

	{#snippet footer()}
		<span class="spacer"></span>
		<button class="btn" onclick={() => close()}>Cancel</button>
		<button
			class="btn primary"
			data-default
			disabled={chosen.length === 0}
			onclick={() => close({ pins: chosen })}>Update {chosen.length}</button
		>
	{/snippet}
</Dialog>

<style>
	.list {
		border: 1px solid var(--line);
	}

	.item + .item {
		border-top: 1px solid var(--line);
	}

	.head {
		width: 100%;
		display: flex;
		align-items: center;
		gap: 12px;
		min-height: 52px;
		padding: 8px 12px;
		text-align: left;
	}

	.head:has(> input) {
		padding: 0 0 0 12px;
	}

	.toggle {
		flex: 1;
		min-width: 0;
		display: flex;
		align-items: center;
		gap: 12px;
		min-height: 52px;
		padding: 8px 12px 8px 0;
		text-align: left;
	}

	.head:not(:has(> input)) > .toggle {
		padding-left: 0;
	}

	.toggle:hover {
		background: var(--hover);
	}

	.all {
		display: flex;
		align-items: center;
		gap: 12px;
		min-height: 36px;
		padding: 0 12px;
		background: var(--band);
		cursor: pointer;
	}

	.off .toggle {
		opacity: 0.5;
	}

	.text {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
		gap: 2px;
	}

	.name {
		font-weight: 500;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.versions {
		flex: none;
		width: 200px;
		display: flex;
		flex-direction: column;
		gap: 2px;
		font-size: 12px;
	}

	.version {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.update {
		color: var(--blue);
		font-weight: 600;
	}

	.update :global(svg) {
		display: inline-block;
		margin-right: 3px;
		vertical-align: -2px;
	}

	.sub {
		min-width: 0;
		display: flex;
		align-items: center;
		gap: 8px;
		color: var(--grey);
	}

	.notes {
		max-height: 360px;
		overflow: auto;
		padding: 4px 16px 12px 56px;
	}

	.problems p {
		margin: 6px 0;
	}
</style>

<script lang="ts">
	import { onMount } from 'svelte'

	import type { BranchChange } from '@/lib/flows/branches'
	import { content } from '@/lib/state/content.svelte'
	import { toasts } from '@/lib/state/toasts.svelte'
	import { ws } from '@/lib/state/workspace.svelte'
	import { loaderVersions, minecraftVersions, search } from '@/lib/tauri/catalog'
	import type { AddItem, CatalogHit, Manifest } from '@/lib/tauri/types'
	import { listNames, loaderName } from '@/lib/util/format'
	import Dialog from '@/ui/Dialog.svelte'
	import ModIcon from '@/ui/ModIcon.svelte'
	import { pending } from '@/ui/pending'
	import Select from '@/ui/Select.svelte'

	interface Props {
		close: (result?: BranchChange) => void
	}

	let { close }: Props = $props()

	let picked = $state<string[]>([])
	let minecraft = $state('')
	let versions = $state<string[]>([])
	let query = $state('')
	let results = $state<CatalogHit[]>([])
	let adding = $state<AddItem[]>([])
	let timer: ReturnType<typeof setTimeout> | null = null

	const updates = $derived(
		content.mods.filter((m) => content.updates[m.key] && m.dependencyType !== 'dependency'),
	)

	onMount(() => {
		void minecraftVersions(false)
			.then((found) => (versions = found))
			.catch((error) => {
				versions = []
				toasts.error(error)
			})
	})

	function find(): void {
		if (timer) clearTimeout(timer)
		const text = query.trim()
		if (!text || !ws.manifest) {
			results = []
			return
		}
		timer = setTimeout(() => {
			void search({
				provider: 'modrinth',
				query: text,
				minecraft: minecraft || ws.manifest!.minecraft,
				loader: ws.manifest!.loader,
				projectType: 'mod',
				offset: 0,
				limit: 8,
			})
				.then((hits) => (results = hits))
				.catch((error) => {
					results = []
					toasts.error(error)
				})
		}, 250)
	}

	function toggleAdd(hit: CatalogHit): void {
		adding = adding.some((a) => a.projectId === hit.projectId)
			? adding.filter((a) => a.projectId !== hit.projectId)
			: [
					...adding,
					{
						provider: 'modrinth',
						projectId: hit.projectId,
						slug: hit.slug,
						name: hit.title,
						projectType: 'mod',
					},
				]
	}

	async function gameChange(): Promise<Partial<Manifest> | null> {
		const manifest = ws.manifest
		if (!minecraft || !manifest) return {}
		if (manifest.loader === 'vanilla') return { minecraft }
		try {
			const newest = (await loaderVersions(manifest.loader, minecraft)).versions[0]
			if (newest) return { minecraft, loaderVersion: newest }
			toasts.error(`${loaderName(manifest.loader)} has no version for Minecraft ${minecraft}`)
		} catch (error) {
			toasts.error(error)
		}
		return null
	}

	async function next(): Promise<void> {
		const game = await gameChange()
		if (!game) return
		const parts = [
			...(picked.length > 0
				? [`update ${listNames(picked.map((id) => content.find(id)?.name ?? id))}`]
				: []),
			...(minecraft ? [`Minecraft ${minecraft}`] : []),
			...(adding.length > 0 ? [`add ${listNames(adding.map((a) => a.name ?? a.projectId))}`] : []),
		]
		close({
			label: parts.length > 0 ? `Try ${parts.join(', ')}` : 'Test',
			pins: picked.map((key) => ({
				key,
				version: content.updates[key]!.versionId,
			})),
			items: adding,
			game: minecraft ? game : undefined,
		})
	}
</script>

<Dialog title="New test branch" width={560} onclose={() => close()}>
	{#if updates.length > 0}
		<div class="band">Updates</div>
		<div class="list">
			{#each updates as mod (mod.key)}
				<label class="row">
					<input
						type="checkbox"
						checked={picked.includes(mod.key)}
						onchange={(e) =>
							(picked = e.currentTarget.checked
								? [...picked, mod.key]
								: picked.filter((key) => key !== mod.key))}
					/>
					<ModIcon url={content.meta[mod.projectId]?.iconUrl} size={20} />
					<span class="name">{mod.name}</span>
					<span class="mono small">→ {content.updates[mod.key]?.versionNumber}</span>
				</label>
			{/each}
		</div>
	{/if}

	<div class="band">Minecraft</div>
	<div class="row">
		<Select
			options={[
				{ value: '', label: `Keep ${ws.manifest?.minecraft ?? ''}` },
				...versions.map((v) => ({ value: v, label: v })),
			]}
			bind:value={minecraft}
			mono
		/>
	</div>

	<div class="band">Content to add</div>
	<div class="row">
		<input
			class="field grow"
			bind:value={query}
			oninput={find}
			placeholder="Search Modrinth"
			spellcheck="false"
		/>
	</div>
	<div class="list">
		{#each results as hit (hit.projectId)}
			<label class="row">
				<input
					type="checkbox"
					checked={adding.some((a) => a.projectId === hit.projectId)}
					onchange={() => toggleAdd(hit)}
				/>
				<ModIcon url={hit.iconUrl} size={20} />
				<span class="name">{hit.title}</span>
				{#if content.holder('modrinth', hit.projectId)}<span class="tag">in pack</span>{/if}
			</label>
		{/each}
		{#each adding.filter((a) => !results.some((r) => r.projectId === a.projectId)) as item (item.projectId)}
			<label class="row">
				<input
					type="checkbox"
					checked
					onchange={() => (adding = adding.filter((a) => a !== item))}
				/>
				<span class="name">{item.name}</span>
			</label>
		{/each}
	</div>

	{#snippet footer()}
		<button class="btn" onclick={() => close()}>Cancel</button>
		<button class="btn primary" data-default onclick={pending(next)}>Next…</button>
	{/snippet}
</Dialog>

<style>
	.list {
		max-height: 200px;
		overflow: auto;
	}

	.row {
		display: flex;
		align-items: center;
		gap: 10px;
		min-height: 32px;
		padding: 2px 0;
	}

	.name {
		flex: 1;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.grow {
		flex: 1;
	}
</style>

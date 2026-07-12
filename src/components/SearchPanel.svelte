<script lang="ts">
	import { Search, Package, FileText, FileSearch, ExternalLink } from '@lucide/svelte'
	import { api, openModrinthPage } from '../api'
	import { store } from '../lib/store.svelte'
	import { contextMenu } from '../lib/contextmenu.svelte'
	import { autofocus } from '../lib/autofocus'
	import { activeSource } from '../types'
	import type { FileMatch, LockedMod } from '../types'

	let { onclose, openFile }: { onclose?: () => void; openFile?: (path: string) => void } = $props()

	let query = $state('')
	let fileMatches = $state<FileMatch[]>([])
	let searching = $state(false)
	let selected = $state(0)

	const modMatches = $derived.by(() => {
		const q = query.trim().toLowerCase()
		if (!q) return []
		return (store.lockfile?.mods ?? [])
			.filter((m) => m.name.toLowerCase().includes(q) || m.slug.toLowerCase().includes(q))
			.sort((a, b) => score(b, q) - score(a, q))
			.slice(0, 20)
	})

	function score(m: LockedMod, q: string): number {
		const n = m.name.toLowerCase()
		if (n === q) return 100
		if (n.startsWith(q)) return 50
		return 10 - n.indexOf(q) * 0.01
	}

	type Item = { kind: 'mod'; mod: LockedMod } | { kind: 'file'; file: FileMatch }
	const items = $derived<Item[]>([
		...modMatches.map((m) => ({ kind: 'mod', mod: m }) as Item),
		...fileMatches.map((f) => ({ kind: 'file', file: f }) as Item),
	])

	$effect(() => {
		items
		selected = 0
	})

	let debounceTimer: ReturnType<typeof setTimeout> | undefined
	$effect(() => {
		const q = query.trim()
		clearTimeout(debounceTimer)
		debounceTimer = setTimeout(async () => {
			if (q.length < 2 || !store.pack) {
				fileMatches = []
				return
			}
			searching = true
			try {
				fileMatches = await api.searchFiles(store.pack.dir, q)
			} catch {
				fileMatches = []
			} finally {
				searching = false
			}
		}, 220)
		return () => clearTimeout(debounceTimer)
	})

	function activate(item: Item) {
		if (item.kind === 'mod') openModrinthPage(item.mod.slug)
		else openFile?.(item.file.path)
	}

	function onKey(e: KeyboardEvent) {
		if (e.key === 'Escape') {
			onclose?.()
		} else if (e.key === 'ArrowDown') {
			e.preventDefault()
			selected = Math.min(selected + 1, items.length - 1)
		} else if (e.key === 'ArrowUp') {
			e.preventDefault()
			selected = Math.max(selected - 1, 0)
		} else if (e.key === 'Enter') {
			const item = items[selected]
			if (item) {
				e.preventDefault()
				activate(item)
			}
		}
	}

	function highlight(text: string): { text: string; hit: boolean }[] {
		const q = query.trim().toLowerCase()
		if (!q) return [{ text, hit: false }]
		const out: { text: string; hit: boolean }[] = []
		let rest = text
		let idx = rest.toLowerCase().indexOf(q)
		while (idx !== -1) {
			if (idx > 0) out.push({ text: rest.slice(0, idx), hit: false })
			out.push({ text: rest.slice(idx, idx + q.length), hit: true })
			rest = rest.slice(idx + q.length)
			idx = rest.toLowerCase().indexOf(q)
		}
		if (rest) out.push({ text: rest, hit: false })
		return out
	}

	const flatIndex = (kind: 'mod' | 'file', i: number) =>
		kind === 'mod' ? i : modMatches.length + i
</script>

<svelte:window onkeydown={onKey} />

<div
	class="fixed inset-0 z-[65] flex justify-center items-start pt-[11vh] bg-[rgba(6,8,12,0.55)] backdrop-blur-[2px]"
	role="presentation"
	onclick={(e) => {
		if (e.target === e.currentTarget) onclose?.()
	}}
>
	<div
		class="w-[640px] max-w-[calc(100vw-2rem)] max-h-[72vh] bg-bg-super-raised border border-divider rounded-lg shadow-floating overflow-hidden flex flex-col"
	>
		<div
			class="flex items-center gap-[0.6rem] px-4 py-[0.9rem] border-b border-divider text-secondary"
		>
			<Search size={18} />
			<input
				bind:value={query}
				class="flex-1 bg-transparent border-0 text-contrast text-[0.95rem] outline-none"
				placeholder="Search mods, file names, and contents…"
				use:autofocus
				spellcheck="false"
			/>
			<kbd
				class="text-[0.66rem] bg-button-bg border border-divider rounded-sm px-[0.35rem] py-[0.1rem] text-secondary"
				>esc</kbd
			>
		</div>

		<div class="overflow-y-auto p-[0.4rem]">
			{#if !query.trim()}
				<div class="text-center text-secondary p-6 text-sm">Search mods and files.</div>
			{:else}
				{#if modMatches.length}
					<div
						class="text-[0.66rem] uppercase tracking-[0.06em] text-secondary font-[650] px-2 pt-[0.6rem] pb-[0.3rem]"
					>
						Mods
					</div>
				{/if}
				{#each modMatches as m, i (m.projectId)}
					<button
						class="flex items-center gap-[0.6rem] w-full text-left bg-transparent border-0 text-body px-[0.6rem] py-2 rounded-md cursor-pointer {selected ===
						flatIndex('mod', i)
							? 'bg-button-bg'
							: ''}"
						onmouseenter={() => (selected = flatIndex('mod', i))}
						onclick={() => openModrinthPage(m.slug)}
						use:contextMenu={() => [
							{ label: 'Open', icon: ExternalLink, onSelect: () => openModrinthPage(m.slug) },
						]}
					>
						<Package size={15} class="text-secondary shrink-0" />
						<span class="font-semibold text-contrast">{m.name}</span>
						<span class="ml-auto text-[0.74rem] text-secondary">{activeSource(m)?.versionNumber}</span
						>
						<ExternalLink size={12} class="text-secondary" />
					</button>
				{/each}

				{#if fileMatches.length}
					<div
						class="text-[0.66rem] uppercase tracking-[0.06em] text-secondary font-[650] px-2 pt-[0.6rem] pb-[0.3rem]"
					>
						Files {#if searching}<span class="opacity-70">· searching…</span>{/if}
					</div>
				{/if}
				{#each fileMatches as f, i (`${f.path}:${f.line}:${i}`)}
					<button
						class="flex items-center gap-[0.6rem] w-full text-left bg-transparent border-0 text-body px-[0.6rem] py-2 rounded-md cursor-pointer {selected ===
						flatIndex('file', i)
							? 'bg-button-bg'
							: ''}"
						onmouseenter={() => (selected = flatIndex('file', i))}
						onclick={() => openFile?.(f.path)}
						use:contextMenu={() => [
							{ label: 'Open', icon: FileText, onSelect: () => openFile?.(f.path) },
						]}
					>
						{#if f.line === 0}
							<FileSearch size={15} class="text-secondary shrink-0" />
						{:else}
							<FileText size={15} class="text-secondary shrink-0" />
						{/if}
						<div class="min-w-0 flex-1">
							<div
								class="text-[0.78rem] text-body font-mono whitespace-nowrap overflow-hidden text-ellipsis"
							>
								{f.path}{#if f.line}<span class="text-secondary">:{f.line}</span>{/if}
								{#if f.line === 0}<span
										class="ml-2 text-[0.66rem] text-brand bg-brand-highlight px-[0.35rem] py-[0.05rem] rounded-sm"
										>name match</span
									>{/if}
							</div>
							{#if f.text}
								<div
									class="text-[0.74rem] text-secondary font-mono whitespace-nowrap overflow-hidden text-ellipsis"
								>
									{#each highlight(f.text) as part, j (j)}<span
											class={part.hit ? 'text-brand font-semibold' : ''}>{part.text}</span
										>{/each}
								</div>
							{/if}
						</div>
					</button>
				{/each}

				{#if !items.length && !searching}
					<div class="text-center text-secondary p-6 text-sm">No matches.</div>
				{/if}
			{/if}
		</div>
	</div>
</div>

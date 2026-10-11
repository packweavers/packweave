<script lang="ts">
	import { Search } from '@lucide/svelte'
	import { onMount, tick } from 'svelte'

	import {
		catalogItems,
		commandItems,
		commitItems,
		contentItems,
		fileItems,
		gitItems,
		GROUP_ORDER,
		instanceItems,
		parseQuery,
		type OpenHow,
		type PaletteItem,
	} from '@/lib/palette'
	import { git } from '@/lib/state/git.svelte'
	import { formatCombo } from '@/lib/state/keymap'
	import { palette } from '@/lib/state/palette.svelte'
	import { toasts } from '@/lib/state/toasts.svelte'
	import { ws } from '@/lib/state/workspace.svelte'
	import { index as indexFiles } from '@/lib/tauri/files'
	import { log as gitLog } from '@/lib/tauri/git'
	import { detectInstances } from '@/lib/tauri/instance'
	import type { DetectedInstance, GitCommit } from '@/lib/tauri/types'
	import { focusOnMount } from '@/ui/focus'
	import Icon from '@/ui/Icon.svelte'

	let fileIndex = $state<string[]>([])
	let commits = $state<GitCommit[]>([])
	let detected = $state<DetectedInstance[]>([])
	let catalog = $state<PaletteItem[]>([])
	let cursor = $state(0)
	let list = $state<HTMLDivElement | null>(null)
	let detecting = false

	const parsed = $derived(parseQuery(palette.query, palette.base))

	onMount(() => {
		const dir = ws.dir
		if (!dir) return
		void indexFiles(dir)
			.then((f) => (fileIndex = f))
			.catch((error) => toasts.error(error))
		if (git.isRepo) {
			void gitLog(dir, 60)
				.then((c) => (commits = c))
				.catch((error) => toasts.error(error))
		}
	})

	$effect(() => {
		if (parsed.mode !== 'instances' || detecting) return
		detecting = true
		void detectInstances()
			.then((d) => (detected = d))
			.catch((error) => toasts.error(error))
	})

	$effect(() => {
		const { mode, text } = parsed
		catalog = []
		if (!ws.isOpen || (mode !== 'catalog' && mode !== 'all') || text.length < 3) return
		const timer = setTimeout(() => {
			void catalogItems(text).then((items) => {
				if (parsed.text === text) catalog = items
			})
		}, 250)
		return () => clearTimeout(timer)
	})

	const items = $derived.by(() => {
		const { mode, text } = parsed
		if (!ws.isOpen) return commandItems(text)
		switch (mode) {
			case 'commands':
				return commandItems(text)
			case 'content':
				return contentItems(text)
			case 'catalog':
				return catalog
			case 'files':
				return fileItems(text, fileIndex)
			case 'git':
				return [...gitItems(text), ...commitItems(text, commits)]
			case 'instances':
				return instanceItems(text, detected)
			case 'goto':
				return [...contentItems(text), ...fileItems(text, fileIndex)]
			default:
				return text
					? [
							...contentItems(text),
							...fileItems(text, fileIndex),
							...commandItems(text).slice(0, 8),
							...gitItems(text),
							...catalog,
						]
					: commandItems('').slice(0, 12)
		}
	})

	const grouped = $derived.by(() => {
		const byGroup = new Map<string, PaletteItem[]>()
		for (const item of items) byGroup.set(item.group, [...(byGroup.get(item.group) ?? []), item])
		return GROUP_ORDER.filter((g) => byGroup.has(g)).map((g) => ({
			group: g,
			items: byGroup.get(g)!,
		}))
	})

	const flat = $derived(grouped.flatMap((g) => g.items))

	function howFrom(event: MouseEvent | KeyboardEvent): OpenHow {
		if (event.metaKey || event.ctrlKey) return 'side'
		return event.altKey ? 'window' : 'open'
	}

	function run(item: PaletteItem, how: OpenHow): void {
		palette.hide()
		item.run(how)
	}

	async function move(by: number): Promise<void> {
		cursor = Math.max(0, Math.min(cursor + by, flat.length - 1))
		await tick()
		list?.querySelector('.active')?.scrollIntoView({ block: 'nearest' })
	}

	function onkeydown(event: KeyboardEvent): void {
		if (event.key === 'Escape') {
			event.preventDefault()
			palette.hide()
		} else if (event.key === 'ArrowDown') {
			event.preventDefault()
			void move(1)
		} else if (event.key === 'ArrowUp') {
			event.preventDefault()
			void move(-1)
		} else if (event.key === 'Enter') {
			event.preventDefault()
			const item = flat[cursor]
			if (item) run(item, howFrom(event))
		}
	}
</script>

<div class="shield" role="presentation" onmousedown={() => palette.hide()}></div>
<div class="palette" role="dialog" aria-label="Search">
	<label class="input-row">
		<Icon icon={Search} size={14} />
		<input
			bind:value={palette.query}
			oninput={() => (cursor = 0)}
			placeholder={ws.isOpen ? 'Search content, files, commands, branches' : 'Run a command'}
			spellcheck="false"
			{onkeydown}
			use:focusOnMount
		/>
		<span class="kbd">esc</span>
	</label>
	<div class="dropdown">
		<div class="results" bind:this={list}>
			{#each grouped as section (section.group)}
				<div class="menu-heading">{section.group}</div>
				{#each section.items as item (item.id)}
					{@const i = flat.indexOf(item)}
					<button
						class="menu-item"
						class:active={i === cursor}
						onmouseenter={() => (cursor = i)}
						onclick={(e) => run(item, howFrom(e))}
					>
						<Icon icon={item.icon} size={14} />
						<span class="ellipsis">{item.label}</span>
						{#if item.detail}<span class="detail ellipsis">{item.detail}</span>{/if}
						{#if item.keys}
							<span class="keys">{item.keys}</span>
						{:else if item.hint && i === cursor}
							<span class="keys">{item.hint}</span>
						{/if}
					</button>
				{/each}
			{:else}
				<div class="none">
					{parsed.mode === 'catalog' && parsed.text.length < 3
						? 'Type to search the catalog'
						: 'No matches'}
				</div>
			{/each}
		</div>
		<div class="footer small">
			<span><span class="kbd">↑↓</span> move</span>
			<span><span class="kbd">{formatCombo('Enter')}</span> do</span>
			<span><span class="kbd">{formatCombo('Mod+Enter')}</span> to the side</span>
			<span><span class="kbd">{formatCombo('Alt+Enter')}</span> new window</span>
			<span class="spacer"></span>
			<span class="mono">&gt; @ + / # !</span>
		</div>
	</div>
</div>

<style>
	@keyframes expand {
		from {
			width: min(380px, 34vw);
		}

		to {
			width: min(600px, 48vw);
		}
	}

	.shield {
		position: fixed;
		inset: 0;
		z-index: 44;
	}

	.palette {
		position: absolute;
		z-index: 45;
		top: 0;
		left: 50%;
		transform: translateX(-50%);
		width: min(600px, 48vw);
		height: 100%;
		animation: expand 140ms ease-out;
	}

	.input-row {
		height: 100%;
		display: flex;
		align-items: center;
		gap: 8px;
		padding-left: 12px;
		background: var(--fill);
	}

	.input-row input {
		flex: 1;
		min-width: 0;
		align-self: stretch;
		font-size: 13px;
	}

	.input-row input:focus-visible {
		outline: none;
	}

	.input-row .kbd {
		align-self: stretch;
		min-width: 40px;
	}

	.dropdown {
		position: absolute;
		top: 100%;
		left: 0;
		right: 0;
		max-height: 70vh;
		display: flex;
		flex-direction: column;
		background: var(--overlay);
		border: 1px solid var(--edge);
		border-top: 0;
	}

	.results {
		flex: 1;
		min-height: 0;
		overflow: auto;
		padding: 4px 0;
	}

	.detail {
		flex: 0 1 auto;
		max-width: 50%;
		font-size: 12px;
	}

	.none {
		padding: 14px;
	}

	.footer {
		flex: none;
		display: flex;
		align-items: center;
		gap: 14px;
		padding: 6px 12px;
		border-top: 1px solid var(--line);
	}
</style>

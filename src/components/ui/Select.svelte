<script lang="ts">
	import { computePosition, flip, shift, offset, size, autoUpdate } from '@floating-ui/dom'
	import { ChevronDown, Check, Search } from '@lucide/svelte'

	let {
		value = $bindable(''),
		options = [],
		placeholder = 'Select…',
		searchable = true,
		display = (o: string) => o,
		valueLabel = '',
		disabled = false,
	}: {
		value?: string
		options?: readonly string[]
		placeholder?: string
		searchable?: boolean
		display?: (o: string) => string
		valueLabel?: string
		disabled?: boolean
	} = $props()

	let open = $state(false)
	let query = $state('')
	let active = $state(0)
	let triggerEl = $state<HTMLButtonElement>()
	let menuEl = $state<HTMLDivElement>()
	let searchEl = $state<HTMLInputElement>()

	const filtered = $derived.by(() => {
		const q = query.trim().toLowerCase()
		if (!q) return options
		return options.filter((o) => o.toLowerCase().includes(q) || display(o).toLowerCase().includes(q))
	})

	function reposition() {
		if (!triggerEl || !menuEl) return
		computePosition(triggerEl, menuEl, {
			placement: 'bottom-start',
			strategy: 'fixed',
			middleware: [
				offset(4),
				flip(),
				shift({ padding: 8 }),
				size({
					apply({ rects, elements }) {
						elements.floating.style.width = `${rects.reference.width}px`
					},
				}),
			],
		}).then(({ x, y }) => {
			if (menuEl) {
				menuEl.style.left = `${x}px`
				menuEl.style.top = `${y}px`
			}
		})
	}

	$effect(() => {
		if (open && triggerEl && menuEl) return autoUpdate(triggerEl, menuEl, reposition)
	})

	function openMenu() {
		open = true
		query = ''
		active = Math.max(0, options.indexOf(value))
		requestAnimationFrame(() => searchEl?.focus())
	}
	function close() {
		open = false
	}
	function pick(o: string) {
		value = o
		open = false
	}
	function move(delta: number) {
		active = Math.min(Math.max(active + delta, 0), filtered.length - 1)
		requestAnimationFrame(() => {
			menuEl?.querySelector('[data-active="true"]')?.scrollIntoView({ block: 'nearest' })
		})
	}
	function onKeydown(e: KeyboardEvent) {
		if (e.key === 'ArrowDown') {
			e.preventDefault()
			move(1)
		} else if (e.key === 'ArrowUp') {
			e.preventDefault()
			move(-1)
		} else if (e.key === 'Enter') {
			e.preventDefault()
			if (filtered.length) pick(filtered[Math.min(active, filtered.length - 1)])
		} else if (e.key === 'Escape') {
			e.preventDefault()
			e.stopPropagation()
			close()
		}
	}
	function onDocClick(e: MouseEvent) {
		if (!open) return
		const t = e.target as Node
		if (triggerEl?.contains(t) || menuEl?.contains(t)) return
		open = false
	}

	function portal(node: HTMLElement) {
		const host = triggerEl?.closest('[data-portal-root]') ?? document.body
		host.appendChild(node)
		return {
			destroy() {
				node.remove()
			},
		}
	}
</script>

<svelte:document onclick={onDocClick} />

<button
	bind:this={triggerEl}
	type="button"
	{disabled}
	onclick={() => (open ? close() : openMenu())}
	class={`flex items-center justify-between gap-2 w-full bg-bg border text-contrast rounded-md px-[0.7rem] py-[0.55rem] text-sm outline-none ${
		disabled
			? 'border-divider opacity-50 cursor-not-allowed'
			: `cursor-pointer ${open ? 'border-brand' : 'border-divider hover:border-divider-dark'}`
	}`}
>
	<span class={`truncate ${value ? '' : 'text-secondary'}`}
		>{value ? (!open && valueLabel ? valueLabel : display(value)) : placeholder}</span
	>
	<ChevronDown size={15} class="text-secondary shrink-0" />
</button>

{#if open}
	<div
		bind:this={menuEl}
		use:portal
		class="fixed z-[100] bg-bg-super-raised border border-divider rounded-md shadow-floating p-1"
		style="left:0;top:0"
	>
		{#if searchable}
			<div class="flex items-center gap-1.5 px-1.5 pb-1 mb-1 border-b border-divider">
				<Search size={13} class="text-secondary shrink-0" />
				<input
					bind:this={searchEl}
					bind:value={query}
					onkeydown={onKeydown}
					oninput={() => (active = 0)}
					placeholder="Search…"
					spellcheck="false"
					class="w-full bg-transparent border-0 outline-none text-[0.84rem] text-contrast py-1"
				/>
			</div>
		{/if}
		<div class="max-h-[14rem] overflow-y-auto">
			{#each filtered as o, i (o)}
				<button
					data-active={i === active}
					class={`flex items-center justify-between gap-2 w-full text-left text-[0.84rem] px-2 py-[0.35rem] rounded-sm cursor-pointer border-none ${
						i === active ? 'bg-button-bg text-contrast' : 'bg-transparent text-body hover:bg-button-bg'
					}`}
					onclick={(e) => {
						e.stopPropagation()
						pick(o)
					}}
					onmouseenter={() => (active = i)}
				>
					<span class="truncate">{display(o)}</span>
					{#if o === value}<Check size={14} class="text-brand shrink-0" />{/if}
				</button>
			{/each}
			{#if !filtered.length}
				<div class="text-secondary text-[0.8rem] px-2 py-2 text-center">No matches</div>
			{/if}
		</div>
	</div>
{/if}

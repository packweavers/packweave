<script lang="ts">
	import { SlidersHorizontal } from '@lucide/svelte'
	import Dropdown from '../ui/Dropdown.svelte'
	import { providerLabel } from '../../lib/mods'

	export interface Facets {
		providers: string[]
		channels: string[]
		sides: string[]
		kinds: string[]
	}

	let {
		facets,
		presentProviders,
	}: {
		facets: Facets
		presentProviders: string[]
	} = $props()

	const facetCount = $derived(
		facets.providers.length + facets.channels.length + facets.sides.length + facets.kinds.length,
	)

	function toggle(group: keyof Facets, value: string) {
		const arr = facets[group]
		const i = arr.indexOf(value)
		if (i >= 0) arr.splice(i, 1)
		else arr.push(value)
	}
	function clear() {
		facets.providers = []
		facets.channels = []
		facets.sides = []
		facets.kinds = []
	}

	const chip = (active: boolean) =>
		`text-[0.74rem] px-2 py-[0.2rem] rounded-max border cursor-pointer ${
			active
				? 'bg-brand border-brand text-on-brand'
				: 'bg-bg-inset border-divider text-secondary hover:text-contrast'
		}`
</script>

<Dropdown placement="bottom-end">
	{#snippet trigger()}
		<button
			class={`inline-flex items-center gap-[0.35rem] bg-button-bg border text-[0.78rem] font-medium px-[0.6rem] py-[0.3rem] rounded-md cursor-pointer hover:text-contrast ${
				facetCount > 0 ? 'text-contrast border-brand' : 'text-secondary border-divider'
			}`}
		>
			<SlidersHorizontal size={14} /> Filters{#if facetCount}<span
					class="bg-brand text-on-brand rounded-max text-[0.62rem] font-semibold px-[0.38rem] py-[0.02rem]"
					>{facetCount}</span
				>{/if}
		</button>
	{/snippet}
	{#snippet content()}
		<div class="w-[230px] flex flex-col gap-[0.6rem] p-[0.2rem]">
			{#if presentProviders.length > 1}
				<div class="flex flex-col gap-[0.35rem]">
					<div class="text-[0.64rem] uppercase tracking-[0.05em] text-secondary font-semibold">
						Source
					</div>
					<div class="flex flex-wrap gap-[0.3rem]">
						{#each presentProviders as p (p)}
							<button class={chip(facets.providers.includes(p))} onclick={() => toggle('providers', p)}>
								{providerLabel(p)}
							</button>
						{/each}
					</div>
				</div>
			{/if}
			<div class="flex flex-col gap-[0.35rem]">
				<div class="text-[0.64rem] uppercase tracking-[0.05em] text-secondary font-semibold">
					Channel
				</div>
				<div class="flex flex-wrap gap-[0.3rem]">
					{#each [['release', 'Release'], ['beta', 'Beta'], ['alpha', 'Alpha']] as c (c[0])}
						<button class={chip(facets.channels.includes(c[0]))} onclick={() => toggle('channels', c[0])}>
							{c[1]}
						</button>
					{/each}
				</div>
			</div>
			<div class="flex flex-col gap-[0.35rem]">
				<div class="text-[0.64rem] uppercase tracking-[0.05em] text-secondary font-semibold">Side</div>
				<div class="flex flex-wrap gap-[0.3rem]">
					{#each [['client', 'Client'], ['server', 'Server'], ['both', 'Both']] as s (s[0])}
						<button class={chip(facets.sides.includes(s[0]))} onclick={() => toggle('sides', s[0])}>
							{s[1]}
						</button>
					{/each}
				</div>
			</div>
			<div class="flex flex-col gap-[0.35rem]">
				<div class="text-[0.64rem] uppercase tracking-[0.05em] text-secondary font-semibold">Kind</div>
				<div class="flex flex-wrap gap-[0.3rem]">
					<button
						class={chip(facets.kinds.includes('explicit'))}
						onclick={() => toggle('kinds', 'explicit')}
					>
						Added by you
					</button>
					<button
						class={chip(facets.kinds.includes('dependency'))}
						onclick={() => toggle('kinds', 'dependency')}
					>
						Dependency
					</button>
				</div>
			</div>
			{#if facetCount}
				<button
					class="self-start bg-transparent border-none text-link text-[0.74rem] cursor-pointer py-[0.1rem] hover:underline"
					onclick={clear}>Clear filters</button
				>
			{/if}
		</div>
	{/snippet}
</Dropdown>

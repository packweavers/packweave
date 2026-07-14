<script lang="ts">
	import { Plug, Sparkles, Package, GitBranch, FolderOpen, ChevronRight } from '@lucide/svelte'
	import type { Component } from 'svelte'
	import Modal from './ui/Modal.svelte'

	type Choice = 'instance' | 'scratch' | 'import' | 'clone' | 'open'

	let { onclose, onpick }: { onclose?: () => void; onpick: (c: Choice) => void } = $props()

	interface Option {
		id: Choice
		icon: Component
		title: string
		desc: string
	}

	const options: Option[] = [
		{
			id: 'instance',
			icon: Plug,
			title: 'From an existing instance',
			desc: 'Build a pack from a Minecraft instance you already play.',
		},
		{
			id: 'scratch',
			icon: Sparkles,
			title: 'Create from scratch',
			desc: 'Start empty and add content yourself.',
		},
		{
			id: 'import',
			icon: Package,
			title: 'Import a pack',
			desc: 'Bring in a Modrinth .mrpack, CurseForge .zip, or Prism / MultiMC instance.',
		},
		{
			id: 'clone',
			icon: GitBranch,
			title: 'Clone from Git',
			desc: 'Open a packweave pack hosted in a Git repository.',
		},
		{
			id: 'open',
			icon: FolderOpen,
			title: 'Open a folder',
			desc: "Open a packweave pack that's already on your computer.",
		},
	]

	function choose(c: Choice) {
		onclose?.()
		onpick(c)
	}
</script>

<Modal title="Start a pack" onclose={() => onclose?.()}>
	<div class="flex flex-col gap-2">
		{#each options as o (o.id)}
			{@const Icon = o.icon}
			<button
				class="group flex items-center gap-[0.85rem] w-full text-left bg-bg-inset border border-divider rounded-md px-[0.85rem] py-[0.75rem] cursor-pointer hover:border-divider-dark hover:bg-brand-highlight"
				onclick={() => choose(o.id)}
			>
				<span
					class="grid place-items-center w-9 h-9 rounded-md bg-bg-raised border border-divider text-secondary shrink-0 group-hover:text-contrast"
				>
					<Icon size={17} />
				</span>
				<span class="flex-1 min-w-0">
					<span class="block text-[0.9rem] font-semibold text-contrast">{o.title}</span>
					<span class="block text-[0.76rem] text-secondary leading-[1.4] mt-[0.1rem]">{o.desc}</span>
				</span>
				<ChevronRight
					size={16}
					class="text-secondary shrink-0 opacity-0 group-hover:opacity-100"
				/>
			</button>
		{/each}
	</div>
</Modal>

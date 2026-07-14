<script lang="ts">
	import { Boxes, Plug, Package } from '@lucide/svelte'
	import type { Component } from 'svelte'
	import ButtonStyled from './ui/ButtonStyled.svelte'
	import { store } from '../lib/store.svelte'

	let { onstart }: { onstart?: () => void } = $props()

	interface Point {
		icon: Component
		title: string
		desc: string
	}

	const points: Point[] = [
		{
			icon: Boxes,
			title: 'Add content',
			desc: 'Search Modrinth and CurseForge. Dependencies resolve automatically.',
		},
		{
			icon: Plug,
			title: 'Sync an instance',
			desc: 'Link a launcher instance to test as you build, and pull changes back in.',
		},
		{
			icon: Package,
			title: 'Publish',
			desc: 'Export a .mrpack or CurseForge .zip, or ship a self-updating pack to your players.',
		},
	]

	function skip() {
		store.setPref('introSeen', true)
	}

	function start() {
		store.setPref('introSeen', true)
		onstart?.()
	}
</script>

<div class="fixed inset-0 z-[75] grid place-items-center bg-black/60 backdrop-blur-[2px]">
	<div
		class="w-[440px] max-w-[calc(100vw-2rem)] bg-bg-super-raised border border-divider rounded-lg shadow-floating p-6"
		role="dialog"
		aria-modal="true"
	>
		<div class="flex flex-col items-center text-center">
			<div class="w-[42px] h-[42px] rounded-[10px] bg-brand rotate-45 mb-[1.1rem]"></div>
			<h2 class="text-[1.25rem]">Welcome to packweave</h2>
			<p class="text-secondary text-[0.86rem] leading-[1.5] mt-[0.5rem] max-w-[340px]">
				packweave builds Minecraft modpacks and keeps the whole thing in a Git repository. Every
				change is tracked, and you can undo, branch, and publish with confidence.
			</p>
		</div>

		<div class="flex flex-col gap-[0.7rem] mt-5 mb-6">
			{#each points as p (p.title)}
				{@const Icon = p.icon}
				<div class="flex items-start gap-[0.7rem]">
					<span
						class="grid place-items-center w-8 h-8 rounded-md bg-bg-inset text-secondary shrink-0 mt-[0.1rem]"
					>
						<Icon size={16} />
					</span>
					<div class="min-w-0">
						<div class="text-[0.85rem] font-semibold text-contrast">{p.title}</div>
						<div class="text-[0.78rem] text-secondary leading-[1.4]">{p.desc}</div>
					</div>
				</div>
			{/each}
		</div>

		<div class="flex items-center justify-end gap-[0.6rem]">
			<ButtonStyled type="transparent" onclick={skip}>Skip</ButtonStyled>
			<ButtonStyled color="brand" onclick={start}>Get started</ButtonStyled>
		</div>
	</div>
</div>

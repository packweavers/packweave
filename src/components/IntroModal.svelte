<script lang="ts">
	import { Boxes, Plug, Package } from '@lucide/svelte'
	import type { Component } from 'svelte'
	import ButtonStyled from './ui/ButtonStyled.svelte'
	import LogoMark from './ui/LogoMark.svelte'
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
			<div class="text-contrast mb-4xl">
				<LogoMark size={44} />
			</div>
			<h2 class="text-display">Welcome to packweave</h2>
			<p class="text-secondary text-regular leading-[1.5] mt-lg max-w-[340px]">
				packweave builds Minecraft modpacks and keeps the whole thing in a Git repository. Every
				change is tracked, and you can undo, branch, and publish with confidence.
			</p>
		</div>

		<div class="flex flex-col gap-3 mt-5 mb-6">
			{#each points as p (p.title)}
				{@const Icon = p.icon}
				<div class="flex items-start gap-3">
					<span
						class="grid place-items-center w-8 h-8 rounded-md bg-bg-inset text-secondary shrink-0 mt-2xs"
					>
						<Icon size={16} />
					</span>
					<div class="min-w-0">
						<div class="text-regular font-semibold text-contrast">{p.title}</div>
						<div class="text-small text-secondary leading-[1.4]">{p.desc}</div>
					</div>
				</div>
			{/each}
		</div>

		<div class="flex items-center justify-end gap-2.5">
			<ButtonStyled type="transparent" onclick={skip}>Skip</ButtonStyled>
			<ButtonStyled color="brand" onclick={start}>Get started</ButtonStyled>
		</div>
	</div>
</div>

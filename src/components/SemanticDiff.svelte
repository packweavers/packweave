<script lang="ts">
	import { Boxes, FileText } from '@lucide/svelte'
	import type { PackDiff, PackItemChange } from '../types'
	import { loaderLabel } from '../util'

	let { diff }: { diff: PackDiff } = $props()

	const LABEL: Record<string, string> = {
		added: 'added',
		removed: 'removed',
		updated: 'updated',
		provider: 'source',
		resided: 'side',
		disabled: 'disabled',
		enabled: 'enabled',
	}

	function detail(c: PackItemChange): string {
		switch (c.kind) {
			case 'added':
				return c.toVersion ?? ''
			case 'removed':
				return c.fromVersion ?? ''
			case 'updated':
				return `${c.fromVersion ?? '?'} → ${c.toVersion ?? '?'}`
			case 'provider':
				return `${c.fromProvider ?? '?'} → ${c.toProvider ?? '?'}`
			case 'resided':
				return `${c.fromSide ?? '?'} → ${c.toSide ?? '?'}`
			case 'disabled':
			case 'enabled':
				return c.toVersion ?? ''
			default:
				return ''
		}
	}

	function envLine(mc: string, loader: string, lv: string | null): string {
		const l = loaderLabel(loader)
		return `${mc} · ${l}${lv ? ` ${lv}` : ''}`
	}

	const kindTag: Record<string, string> = {
		added: 'bg-[color-mix(in_srgb,var(--color-green)_18%,transparent)] text-green',
		removed: 'bg-[color-mix(in_srgb,var(--color-red)_16%,transparent)] text-red',
		updated: 'bg-[color-mix(in_srgb,var(--color-blue)_16%,transparent)] text-blue',
		enabled: 'bg-[color-mix(in_srgb,var(--color-green)_18%,transparent)] text-green',
		disabled: 'bg-[color-mix(in_srgb,var(--color-orange)_16%,transparent)] text-orange',
	}

	const tagWidth = (kind: string) => (kind === 'disabled' || kind === 'enabled' ? 'w-[5rem]' : '')
	const baseTag =
		'shrink-0 w-[4.4rem] text-center text-micro font-bold uppercase tracking-[0.03em] py-2xs rounded-sm'
</script>

<div class="flex flex-col gap-px">
	{#if !diff.items.length && !diff.env && !diff.files.length}
		<div class="text-secondary text-regular px-1 py-2">No pack changes.</div>
	{/if}

	{#if diff.env}
		<div class="flex items-center gap-2 px-md py-sm rounded-sm hover:bg-bg-raised">
			<span
				class="{baseTag} bg-[color-mix(in_srgb,var(--color-orange)_16%,transparent)] text-orange"
				>env</span
			>
			<Boxes class="shrink-0 text-secondary" size={13} />
			<span
				class="flex-1 min-w-[3rem] font-semibold text-contrast text-regular whitespace-nowrap overflow-hidden text-ellipsis"
				>Minecraft &amp; loader</span
			>
			<span
				class="flex-[0_1_auto] min-w-0 max-w-[62%] text-right text-tiny text-secondary font-mono whitespace-nowrap overflow-hidden text-ellipsis"
			>
				{envLine(diff.env.fromMinecraft, diff.env.fromLoader, diff.env.fromLoaderVersion)}
				→
				{envLine(diff.env.toMinecraft, diff.env.toLoader, diff.env.toLoaderVersion)}
			</span>
		</div>
	{/if}

	{#each diff.items as c (c.projectType + '/' + c.slug)}
		<div class="flex items-center gap-2 px-md py-sm rounded-sm hover:bg-bg-raised">
			<span class="{baseTag} {tagWidth(c.kind)} {kindTag[c.kind] ?? 'bg-button-bg text-secondary'}"
				>{LABEL[c.kind] ?? c.kind}</span
			>
			<span
				class="flex-1 min-w-[3rem] font-semibold text-contrast text-regular whitespace-nowrap overflow-hidden text-ellipsis"
				title={c.name}>{c.name}</span
			>
			<span
				class="flex-[0_1_auto] min-w-0 max-w-[62%] text-right text-tiny text-secondary font-mono whitespace-nowrap overflow-hidden text-ellipsis"
				title={detail(c)}>{detail(c)}</span
			>
		</div>
	{/each}

	{#if diff.files.length}
		<div
			class="flex items-center gap-1.5 text-tiny text-secondary p-md mt-1 border-t border-divider"
		>
			<FileText size={12} />
			{diff.files.length} file{diff.files.length === 1 ? '' : 's'} changed
		</div>
	{/if}
</div>

<script lang="ts" module>
	import { langFromName } from '../../highlight'

	export function isConfigForm(name: string): boolean {
		return ['json', 'json5', 'toml', 'properties'].includes(langFromName(name))
	}
</script>

<script lang="ts">
	import type { Lang } from '../../highlight'
	import {
		parseConfigLines,
		applyEntry,
		flattenJson,
		setByPath,
		type LineEntry,
		type JsonLeaf,
		type JsonValue,
		type ScalarType,
	} from '../../lib/configform'

	let {
		value = $bindable(''),
		lang,
		onsave,
	}: { value?: string; lang: Lang; onsave?: () => void } = $props()

	let rootEl = $state<HTMLDivElement>()
	function onFocusOut(e: FocusEvent) {
		const next = e.relatedTarget as Node | null
		if (next && rootEl?.contains(next)) return
		onsave?.()
	}

	const isJson = $derived(lang === 'json' || lang === 'json5')

	const parsed = $derived.by(() => {
		if (isJson) {
			try {
				const json = JSON.parse(value || '{}') as JsonValue
				return { ok: true as const, mode: 'json' as const, json, leaves: flattenJson(json) }
			} catch (e) {
				return { ok: false as const, error: (e as Error).message }
			}
		}
		const { lines, entries } = parseConfigLines(value ?? '', lang)
		return { ok: true as const, mode: 'lines' as const, lines, entries }
	})

	const groups = $derived.by(() => {
		if (!parsed.ok || parsed.mode !== 'lines') return []
		const map = new Map<string, LineEntry[]>()
		for (const e of parsed.entries) {
			const a = map.get(e.table) ?? []
			a.push(e)
			map.set(e.table, a)
		}
		return [...map.entries()].map(([table, entries]) => ({ table, entries }))
	})

	function updateLine(entry: LineEntry, v: string | number | boolean) {
		if (!parsed.ok || parsed.mode !== 'lines') return
		value = applyEntry(parsed.lines, lang, entry, v)
	}
	function updateJson(leaf: JsonLeaf, v: string | number | boolean | null) {
		if (!parsed.ok || parsed.mode !== 'json') return
		const clone = JSON.parse(JSON.stringify(parsed.json)) as JsonValue
		setByPath(clone, leaf.path, v)
		value = JSON.stringify(clone, null, 2) + '\n'
	}

	const inputClass =
		'bg-bg border border-divider rounded-sm px-2 py-[0.3rem] text-[0.82rem] text-contrast outline-none focus:border-brand w-full'
</script>

{#snippet control(
	type: ScalarType | 'null',
	val: string | number | boolean | null,
	commit: (v: string | number | boolean) => void,
)}
	{#if type === 'bool'}
		<input
			type="checkbox"
			class="w-[1.05rem] h-[1.05rem] accent-brand cursor-pointer"
			checked={val === true}
			onchange={(e) => commit((e.target as HTMLInputElement).checked)}
		/>
	{:else if type === 'number'}
		<input
			type="number"
			class={inputClass}
			value={val as number}
			onchange={(e) => commit(Number((e.target as HTMLInputElement).value))}
		/>
	{:else}
		<input
			type="text"
			spellcheck="false"
			class={`${inputClass} font-mono`}
			value={val === null ? '' : String(val)}
			onchange={(e) => commit((e.target as HTMLInputElement).value)}
		/>
	{/if}
{/snippet}

<div bind:this={rootEl} onfocusout={onFocusOut} class="flex-1 min-h-0 overflow-y-auto bg-bg-inset px-4 py-3">
	{#if !parsed.ok}
		<div class="grid place-items-center h-full text-center text-secondary text-[0.85rem] gap-1">
			<span>Can't show this as a form.</span>
			<span class="text-[0.75rem] opacity-70">{parsed.error}. Switch to Text to edit it.</span>
		</div>
	{:else if parsed.mode === 'json'}
		{#if parsed.leaves.length === 0}
			<div class="grid place-items-center h-full text-secondary text-[0.85rem]">No editable fields.</div>
		{:else}
			<div class="max-w-[640px] mx-auto flex flex-col">
				{#each parsed.leaves as leaf (leaf.path.join('.'))}
					<div class="flex items-center gap-3 py-[0.35rem] border-b border-divider/60">
						<span class="flex-1 min-w-0 text-[0.8rem] text-body font-mono whitespace-nowrap overflow-hidden text-ellipsis">
							{leaf.path.join(' › ') || '(value)'}
						</span>
						<div class="w-[15rem] shrink-0 flex justify-end">
							{@render control(leaf.type, leaf.value, (v) => updateJson(leaf, v))}
						</div>
					</div>
				{/each}
			</div>
		{/if}
	{:else if groups.length === 0}
		<div class="grid place-items-center h-full text-secondary text-[0.85rem]">No editable fields.</div>
	{:else}
		<div class="max-w-[640px] mx-auto flex flex-col gap-1">
			{#each groups as g (g.table)}
				{#if g.table}
					<div class="text-[0.7rem] uppercase tracking-[0.05em] text-secondary font-semibold mt-3 mb-[0.1rem]">
						{g.table}
					</div>
				{/if}
				{#each g.entries as entry (entry.line)}
					<div class="flex items-start gap-3 py-[0.4rem] border-b border-divider/60">
						<div class="flex-1 min-w-0">
							<span
								class="block text-[0.8rem] text-body font-mono whitespace-nowrap overflow-hidden text-ellipsis"
							>
								{entry.key}
							</span>
							{#if entry.comment}
								<div class="text-[0.7rem] text-secondary mt-[0.15rem] whitespace-pre-wrap leading-[1.4]">
									{entry.comment}
								</div>
							{/if}
						</div>
						<div class="w-[15rem] shrink-0 flex justify-end pt-[0.1rem]">
							{@render control(entry.type, entry.value, (v) => updateLine(entry, v))}
						</div>
					</div>
				{/each}
			{/each}
		</div>
	{/if}
</div>

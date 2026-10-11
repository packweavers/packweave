<script lang="ts">
	import { Lock, LockOpen } from '@lucide/svelte'
	import { tick } from 'svelte'

	import {
		applyEntry,
		jsonLeaves,
		jsonProblem,
		parseConfigLines,
		replaceJsonLeaf,
		type ScalarType,
	} from '@/lib/files/configform'
	import type { Lang } from '@/lib/files/highlight'
	import { preserve } from '@/lib/state/preserve.svelte'
	import Empty from '@/ui/Empty.svelte'
	import Icon from '@/ui/Icon.svelte'

	interface Props {
		text: string
		lang: Lang
		baseline: string | null
		keepPath: string | null
		revealKey?: string | null
		onchange: (text: string) => void
	}

	let { text, lang, baseline, keepPath, revealKey = null, onchange }: Props = $props()

	let form = $state<HTMLDivElement>()

	const wholeFile = $derived(keepPath ? preserve.fileUnlocked(keepPath) : false)

	type Value = string | number | boolean | null

	interface Field {
		id: string
		group: string
		key: string
		label: string
		type: ScalarType | 'null'
		value: Value
		hint: string
		commit: (value: Value) => void
	}

	function humanize(key: string): string {
		const words = key
			.replace(/([a-z0-9])([A-Z])/g, '$1 $2')
			.replace(/[_\-.]+/g, ' ')
			.trim()
			.toLowerCase()
		return words.charAt(0).toUpperCase() + words.slice(1)
	}

	function readValues(source: string): Map<string, Value> {
		const map = new Map<string, Value>()
		if (lang === 'json' || lang === 'json5') {
			for (const leaf of jsonLeaves(source)) map.set(leaf.id, leaf.value)
			return map
		}
		for (const entry of parseConfigLines(source, lang).entries) {
			map.set(entry.table ? `${entry.table}.${entry.key}` : entry.key, entry.value)
		}
		return map
	}

	const parsed = $derived.by((): { fields: Field[]; error: string | null } => {
		if (lang === 'json' || lang === 'json5') {
			const problem = jsonProblem(text)
			if (problem) return { fields: [], error: problem }
			const fields = jsonLeaves(text).map((leaf) => {
				const path = leaf.id.split('.')
				return {
					id: leaf.id,
					group: path.length > 1 ? path[0] : '',
					key: leaf.id,
					label: humanize(path[path.length - 1] ?? ''),
					type: leaf.type,
					value: leaf.value,
					hint: '',
					commit: (value: Value) => onchange(replaceJsonLeaf(text, leaf, value)),
				}
			})
			return { fields, error: null }
		}
		const { lines, entries } = parseConfigLines(text, lang)
		const fields = entries.map((entry) => ({
			id: `${entry.line}`,
			group: entry.table,
			key: entry.table ? `${entry.table}.${entry.key}` : entry.key,
			label: humanize(entry.key),
			type: entry.type,
			value: entry.value,
			hint: entry.comment,
			commit: (value: Value) => onchange(applyEntry(lines, lang, entry, value ?? '')),
		}))
		return { fields, error: null }
	})

	const before = $derived(baseline === null ? null : readValues(baseline))
	const groups = $derived.by(() => {
		const map = new Map<string, Field[]>()
		for (const field of parsed.fields) {
			map.set(field.group, [...(map.get(field.group) ?? []), field])
		}
		return [...map.entries()]
	})

	$effect(() => {
		const key = revealKey
		if (!key || !form || parsed.fields.length === 0) return
		void tick().then(() =>
			form?.querySelector(`[data-key="${CSS.escape(key)}"]`)?.scrollIntoView({ block: 'center' }),
		)
	})

	function keepToggle(field: Field): void {
		if (!keepPath) return
		void preserve.setSetting(keepPath, field.key, !preserve.settingUnlocked(keepPath, field.key))
	}

	function was(field: Field): string | null {
		if (!before || !before.has(field.key)) return null
		const old = before.get(field.key)
		return old === field.value ? null : String(old)
	}
</script>

{#if parsed.error}
	<Empty title="Can't show this as a form" detail={parsed.error} />
{:else if parsed.fields.length === 0}
	<Empty title="No settings in this file" />
{:else}
	{#if wholeFile}
		<div class="banner">
			Unlocked: players can edit this file. Locked settings keep the pack's value.
		</div>
	{/if}
	<div class="scroll form" bind:this={form}>
		{#each groups as [group, fields] (group)}
			{#if group}<div class="band bar">{humanize(group)}</div>{/if}
			{#each fields as field (field.id)}
				{@const old = was(field)}
				<label class="setting" data-key={field.key} class:revealed={field.key === revealKey}>
					<span class="text">
						<span class="medium">{field.label}</span>
						<span class="mono small">{field.key}{old !== null ? ` · was ${old}` : ''}</span>
						{#if field.hint}<span class="small hint">{field.hint}</span>{/if}
					</span>
					{#if keepPath}
						{@const kept = preserve.settingUnlocked(keepPath, field.key)}
						<button
							class="keep"
							class:on={kept}
							title={kept ? 'Unlocked: players can edit' : 'Locked: players cannot edit'}
							aria-pressed={kept}
							onclick={(e) => {
								e.preventDefault()
								keepToggle(field)
							}}
						>
							<Icon icon={kept ? LockOpen : Lock} size={14} />
						</button>
					{/if}
					<span class="control">
						{#if field.type === 'bool'}
							<input
								type="checkbox"
								class="switch"
								checked={field.value === true}
								onchange={(e) => field.commit(e.currentTarget.checked)}
							/>
						{:else if field.type === 'number'}
							<input
								class="field mono"
								type="number"
								value={field.value as number}
								onchange={(e) => field.commit(Number(e.currentTarget.value))}
							/>
						{:else}
							<input
								class="field mono"
								value={field.value === null ? '' : String(field.value)}
								spellcheck="false"
								onchange={(e) => field.commit(e.currentTarget.value)}
							/>
						{/if}
					</span>
				</label>
			{/each}
		{/each}
	</div>
{/if}

<style>
	.form {
		padding-bottom: 32px;
	}

	.setting {
		display: flex;
		align-items: center;
		gap: 16px;
		min-height: 52px;
		padding: 8px 16px;
		border-bottom: 1px solid var(--line);
	}

	.setting.revealed {
		background: var(--hit-bg);
	}

	.text {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
		gap: 2px;
	}

	.hint {
		white-space: pre-wrap;
	}

	.control {
		flex: 0 0 240px;
		display: flex;
		justify-content: flex-end;
	}

	.control .field {
		width: 100%;
	}

	.keep {
		width: 28px;
		height: 28px;
		display: grid;
		place-items: center;
		flex: none;
		color: var(--grey);
		opacity: 0.4;
	}

	.setting:hover .keep,
	.keep.on {
		opacity: 1;
	}

	.keep.on {
		color: var(--blue);
	}

	.keep:hover {
		background: var(--fill);
	}
</style>

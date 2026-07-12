<script lang="ts">
	import ButtonStyled from '../ui/ButtonStyled.svelte'
	import MarkdownEditor from '../ui/MarkdownEditor.svelte'
	import Select from '../ui/Select.svelte'
	import { api } from '../../api'
	import { store } from '../../lib/store.svelte'
	import { GIT_EMPTY_TREE as EMPTY_TREE } from '../../util'
	import type { GitCommit, Tag } from '../../types'

	let {
		commits,
		tags,
		onsaved,
	}: {
		commits: GitCommit[]
		tags: Tag[]
		onsaved: () => void
	} = $props()

	let clFrom = $state('')
	let clTo = $state('HEAD')
	let clHeading = $state('')
	let clText = $state('')
	let clLoading = $state(false)
	let clCopied = $state(false)
	let clLinks = $state(false)
	let latestTag = $state<string | null>(null)

	const refOptions = $derived.by(() => {
		const opts: { value: string; label: string }[] = [
			{ value: 'HEAD', label: 'HEAD (latest commit)' },
			{ value: EMPTY_TREE, label: 'Start (empty)' },
		]
		for (const t of tags) opts.push({ value: t.name, label: `tag: ${t.name}` })
		for (const cm of commits) opts.push({ value: cm.hash, label: `${cm.short} · ${cm.subject}` })
		return opts
	})

	let lastGenerated = $state('')

	function generate(forFile: boolean): Promise<string> {
		const heading = clHeading.trim() || undefined
		return clTo === '__working__'
			? api.changelogWorking(store.pack!.dir, heading, clLinks, forFile)
			: api.changelogBetween(store.pack!.dir, clFrom || EMPTY_TREE, clTo, heading, clLinks, forFile)
	}

	async function genChangelog() {
		if (!store.pack) return
		clLoading = true
		try {
			clText = await generate(false)
			lastGenerated = clText
		} catch (e) {
			clText = `error: ${e}`
		} finally {
			clLoading = false
		}
	}

	$effect(() => {
		if (!store.pack) return
		api
			.gitLatestTag(store.pack.dir)
			.then((t) => {
				latestTag = t
				if (!clFrom) clFrom = t ?? (commits.length ? commits[commits.length - 1].hash : EMPTY_TREE)
				if (!clText) void genChangelog()
			})
			.catch(() => {
				latestTag = null
				if (!clFrom) clFrom = commits.length ? commits[commits.length - 1].hash : EMPTY_TREE
				if (!clText) void genChangelog()
			})
	})

	function copyChangelog() {
		navigator.clipboard?.writeText(clText)
		clCopied = true
		setTimeout(() => (clCopied = false), 1500)
	}

	async function saveChangelog() {
		if (!store.pack || !clText.trim()) return
		try {
			const body = clText === lastGenerated ? await generate(true) : clText
			await api.changelogSave(store.pack.dir, body)
			store.notify('success', 'Saved to CHANGELOG.md')
			onsaved()
		} catch (e) {
			store.notify('error', `${e}`)
		}
	}

	function onSaveKey(e: KeyboardEvent) {
		if ((e.metaKey || e.ctrlKey) && (e.key === 's' || e.key === 'S')) {
			e.preventDefault()
			if (clText.trim() && !store.busy) saveChangelog()
		}
	}

	const baBtn =
		'inline-flex items-center gap-1 bg-bg-inset border border-divider text-body text-[0.72rem] px-[0.45rem] py-[0.2rem] rounded-sm cursor-pointer hover:text-contrast hover:border-divider-dark'
</script>

<svelte:window onkeydown={onSaveKey} />

<div class="col-span-2 flex flex-col min-h-0 p-3 gap-2.5 overflow-y-auto">
	<div class="flex flex-wrap items-end gap-2.5">
		<div class="flex flex-col gap-1 min-w-[13rem]">
			<span class="text-[0.64rem] uppercase tracking-[0.05em] text-secondary font-[650]">From</span>
			<Select
				bind:value={clFrom}
				options={refOptions.map((o) => o.value)}
				display={(v) => refOptions.find((o) => o.value === v)?.label ?? v}
			/>
		</div>
		<div class="flex flex-col gap-1 min-w-[13rem]">
			<span class="text-[0.64rem] uppercase tracking-[0.05em] text-secondary font-[650]">To</span>
			<Select
				bind:value={clTo}
				options={['__working__', ...refOptions.map((o) => o.value)]}
				display={(v) =>
					v === '__working__'
						? 'Working tree (uncommitted)'
						: (refOptions.find((o) => o.value === v)?.label ?? v)}
			/>
		</div>
		<label
			class="flex flex-col gap-1 flex-1 min-w-[8rem] text-[0.64rem] uppercase tracking-[0.05em] text-secondary font-[650]"
		>
			Heading
			<input
				bind:value={clHeading}
				placeholder="v1.2.0"
				spellcheck="false"
				class="w-full bg-bg-inset border border-divider text-contrast rounded-sm px-2 py-[0.35rem] text-[0.8rem] outline-none focus:border-brand normal-case font-normal"
			/>
		</label>
		<ButtonStyled color="brand" size="small" disabled={clLoading} onclick={genChangelog}>
			{clLoading ? 'Generating…' : 'Generate'}
		</ButtonStyled>
	</div>

	<div class="flex flex-wrap gap-1.5">
		<button
			class="{baBtn} disabled:opacity-40 disabled:cursor-default"
			disabled={!latestTag}
			onclick={() => {
				if (latestTag) {
					clFrom = latestTag
					clTo = 'HEAD'
					genChangelog()
				}
			}}>Since last release{latestTag ? ` (${latestTag})` : ''}</button
		>
		<button
			class={baBtn}
			onclick={() => {
				clFrom = EMPTY_TREE
				clTo = 'HEAD'
				genChangelog()
			}}>Whole pack</button
		>
		<button
			class={baBtn}
			onclick={() => {
				clTo = '__working__'
				genChangelog()
			}}>Working vs HEAD</button
		>
		<label
			class="ml-auto flex items-center gap-[0.35rem] text-[0.72rem] text-secondary cursor-pointer normal-case font-normal tracking-normal"
		>
			<input
				type="checkbox"
				class="w-[0.85rem] h-[0.85rem] accent-brand cursor-pointer"
				checked={clLinks}
				onchange={() => {
					clLinks = !clLinks
					genChangelog()
				}}
			/>
			Link mods
		</label>
	</div>

	<MarkdownEditor
		bind:value={clText}
		placeholder="Generate a changelog between two points, or write your own…"
	/>

	<div class="flex items-center gap-2">
		<ButtonStyled size="small" type="outlined" disabled={!clText.trim()} onclick={copyChangelog}>
			{clCopied ? 'Copied' : 'Copy'}
		</ButtonStyled>
		<ButtonStyled
			size="small"
			type="outlined"
			disabled={!clText.trim() || store.busy}
			onclick={saveChangelog}>Save to CHANGELOG.md</ButtonStyled
		>
		<span class="text-[0.72rem] text-secondary"
			>Disable/enable shown here; saved to the file as removed/added · ⌘S saves</span
		>
	</div>
</div>

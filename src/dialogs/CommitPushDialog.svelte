<script lang="ts">
	import { untrack } from 'svelte'

	import { contentChange } from '@/lib/content/changes'
	import { changeMark, changeWord, displayPath, isContentPath } from '@/lib/git/changes'
	import { content } from '@/lib/state/content.svelte'
	import { git } from '@/lib/state/git.svelte'
	import { plural } from '@/lib/util/format'
	import Dialog from '@/ui/Dialog.svelte'
	import TextField from '@/ui/TextField.svelte'

	interface Props {
		title: string
		message: string
		selected: string[]
		close: (result?: { message: string; files: string[] } | 'skip') => void
	}

	let { title, message, selected, close }: Props = $props()

	let text = $state(untrack(() => message))
	let picked = $state<Record<string, boolean>>(
		untrack(() => Object.fromEntries(selected.map((path) => [path, true]))),
	)

	const items = $derived(content.changes?.items ?? [])
	const files = $derived(git.changes.filter((c) => !isContentPath(c.path) && !c.conflicted))
	const paths = $derived([...items.map((i) => i.path), ...files.map((f) => f.path)])
	const chosen = $derived(paths.filter((path) => picked[path]))
	const all = $derived(paths.length > 0 && chosen.length === paths.length)
	const ahead = $derived(git.status?.ahead ?? 0)
	const unpushed = $derived(ahead > 0 || !git.status?.upstream)

	function pickAll(on: boolean): void {
		picked = Object.fromEntries(paths.map((path) => [path, on]))
	}

	function submit(): void {
		close({ message: text.trim() || message, files: chosen })
	}
</script>

<Dialog {title} width={560} onclose={() => close()}>
	{#if paths.length > 0}
		<div class="form-field">
			<span class="label">Message</span>
			<TextField bind:value={text} placeholder={message} />
		</div>
		<div class="list">
			<label class="trow all">
				<input
					type="checkbox"
					checked={all}
					indeterminate={chosen.length > 0 && !all}
					onchange={(e) => pickAll(e.currentTarget.checked)}
				/>
				<span class="name">Select all</span>
				<span class="spacer"></span>
				<span class="small">{chosen.length} of {paths.length}</span>
			</label>
			{#each items as item (item.path)}
				{@const word = contentChange(item.kind)}
				<label class="trow">
					<input
						type="checkbox"
						checked={picked[item.path] === true}
						onchange={(e) => (picked[item.path] = e.currentTarget.checked)}
					/>
					<span class="name ellipsis">{item.name}</span>
					<span class="spacer"></span>
					<span class="mark-text {word}">{changeMark(word)}</span>
				</label>
			{/each}
			{#each files as file (file.path)}
				{@const word = changeWord(file)}
				<label class="trow">
					<input
						type="checkbox"
						checked={picked[file.path] === true}
						onchange={(e) => (picked[file.path] = e.currentTarget.checked)}
					/>
					<span class="name ellipsis mono">{displayPath(file.path)}</span>
					<span class="spacer"></span>
					<span class="mark-text {word}">{changeMark(word)}</span>
				</label>
			{/each}
		</div>
	{/if}
	{#if ahead > 0}
		<p class="small hint">
			{chosen.length > 0 ? 'Also pushes' : 'Pushes'}
			{plural(ahead, 'earlier commit', 'earlier commits')}.
		</p>
	{/if}
	{#snippet footer()}
		<button class="btn" onclick={() => close('skip')}>Skip</button>
		<span class="spacer"></span>
		<button class="btn" onclick={() => close()}>Cancel</button>
		<button class="btn primary" disabled={chosen.length === 0 && !unpushed} onclick={submit}>
			{chosen.length > 0 ? 'Commit and push' : 'Push'}
		</button>
	{/snippet}
</Dialog>

<style>
	.list {
		max-height: 240px;
		overflow-y: auto;
		border: 1px solid var(--line);
		border-bottom: 0;
	}

	.trow {
		cursor: pointer;
	}

	.all {
		background: var(--band);
	}
</style>

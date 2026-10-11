<script lang="ts">
	import { onMount, untrack } from 'svelte'

	import { settingLine } from '@/lib/files/configform'
	import { formatCode } from '@/lib/files/editing'
	import { langOf } from '@/lib/files/highlight'
	import { fileKind, type FileKind } from '@/lib/files/kind'
	import { pathHints } from '@/lib/files/paths'
	import { createMissingFile, ignoreKindOf } from '@/lib/flows/files'
	import { keysFor } from '@/lib/state/commands.svelte'
	import { files } from '@/lib/state/files.svelte'
	import { fileView, type FileMode } from '@/lib/state/fileview.svelte'
	import { git } from '@/lib/state/git.svelte'
	import { PRESERVE_FILE } from '@/lib/state/preserve.svelte'
	import { tabs, type Doc } from '@/lib/state/tabs.svelte'
	import { tasks } from '@/lib/state/tasks.svelte'
	import { toasts } from '@/lib/state/toasts.svelte'
	import { ws } from '@/lib/state/workspace.svelte'
	import { errorText } from '@/lib/tauri/call'
	import { readImage, readNbt } from '@/lib/tauri/files'
	import { diffFile, showFile } from '@/lib/tauri/git'
	import { setPackIcon } from '@/lib/tauri/pack'
	import type { NbtNode } from '@/lib/tauri/types'
	import { openInDefaultApp, reveal } from '@/lib/tauri/windows'
	import { bytes } from '@/lib/util/format'
	import { join } from '@/lib/util/path'
	import { fileManager } from '@/lib/util/platform'
	import CodeEditor from '@/ui/CodeEditor.svelte'
	import DiffView from '@/ui/DiffView.svelte'
	import Empty from '@/ui/Empty.svelte'
	import ImageView from '@/ui/ImageView.svelte'
	import Markdown from '@/ui/Markdown.svelte'
	import NbtView from '@/ui/NbtView.svelte'
	import { pending } from '@/ui/pending'
	import Segmented from '@/ui/Segmented.svelte'

	import ConfigForm from './file/ConfigForm.svelte'

	interface Props {
		doc: Doc
	}

	let { doc }: Props = $props()

	type Mode = FileMode

	const rel = $derived(doc.arg)
	const buffer = $derived(files.buffers[rel])
	const saved = $derived(buffer?.state === 'text' ? buffer.original : undefined)
	const kind = $derived(fileKind(rel, saved))
	const lang = $derived(langOf(rel, saved))
	const dirty = $derived(files.isDirty(rel))
	const crumbs = $derived(rel.replace(/^overrides\//, '').split('/'))
	const changed = $derived(git.changes.some((c) => c.path === rel))
	const head = $derived(git.status?.head ?? null)

	let mode = $state<Mode>(untrack(() => fileView.modeFor(kind, defaultMode(kind))))
	let modeKind = $state<FileKind>(untrack(() => kind))

	$effect(() => {
		const now = kind
		if (now === untrack(() => modeKind)) return
		modeKind = now
		mode = fileView.modeFor(now, defaultMode(now))
	})
	let baseline = $state<string | null>(null)
	let diff = $state('')
	let image = $state<string | null>(null)
	let imageSize = $state('')
	let zoom = $state<'fit' | '1' | '2'>('fit')
	let nbt = $state<NbtNode | null>(null)
	let failed = $state<string | null>(null)
	let hintPaths = $state<string[] | null>(null)
	const target = $derived(files.reveal?.rel === rel ? files.reveal : null)
	const revealKey = $derived(target?.key ?? null)
	const revealLine = $derived.by(() => {
		if (!target) return null
		if (target.key === undefined) return target.line
		return buffer?.state === 'text' ? settingLine(buffer.text, lang, target.key) : null
	})

	onMount(() => {
		const dir = ws.dir
		if (!dir) return
		if (kind === 'image') {
			void readImage(dir, rel)
				.then((src) => (image = src))
				.catch((e) => (failed = errorText(e)))
		} else if (kind === 'nbt') {
			void readNbt(dir, rel)
				.then((root) => (nbt = root))
				.catch((e) => (failed = errorText(e)))
		} else {
			void files.open(rel)
		}
	})

	$effect(() => {
		if (!ws.dir) return
		if (ignoreKindOf(rel)) {
			void files.packPaths().then((found) => (hintPaths = found))
		} else if (rel === PRESERVE_FILE) {
			void files.gamePaths().then((found) => (hintPaths = found))
		}
	})

	const hints = $derived(hintPaths ? (line: string) => pathHints(hintPaths ?? [], line) : null)

	$effect(() => {
		const dir = ws.dir
		const at = head
		if (!dir || !at || kind === 'image' || kind === 'nbt') {
			baseline = null
			return
		}
		void showFile(dir, 'HEAD', rel)
			.then((text) => {
				if (head === at) baseline = text
			})
			.catch((error) => {
				baseline = null
				tasks.fail(`Reading the committed ${rel}`, error)
			})
	})

	$effect(() => {
		const dir = ws.dir
		void buffer?.original
		void git.status
		if (mode !== 'compare' || !dir) return
		void diffFile(dir, rel)
			.then((d) => (diff = d))
			.catch((error) => tasks.fail('Reading the diff', error))
	})

	function defaultMode(of: FileKind): Mode {
		if (of === 'config') return 'form'
		return of === 'markdown' ? 'edit' : 'text'
	}

	function edit(text: string): void {
		files.edit(rel, text)
		if (doc.preview) tabs.keep(doc.id)
	}

	function format(): void {
		if (!buffer) return
		edit(formatCode(buffer.text, lang))
	}

	async function useAsIcon(): Promise<void> {
		if (!ws.dir) return
		try {
			await setPackIcon(ws.dir, join(ws.dir, rel))
			await ws.refreshIcon()
			ws.changed()
			toasts.show('Pack icon set')
		} catch (error) {
			toasts.error(error)
		}
	}

	const modeOptions = $derived.by(() => {
		if (kind === 'config') {
			return [
				{ value: 'form' as Mode, label: 'Form' },
				{ value: 'text' as Mode, label: 'Text' },
				...(changed ? [{ value: 'compare' as Mode, label: 'Compare' }] : []),
			]
		}
		if (kind === 'markdown') {
			return [
				{ value: 'edit' as Mode, label: 'Edit' },
				{ value: 'preview' as Mode, label: 'Preview' },
				{ value: 'split' as Mode, label: 'Split' },
			]
		}
		return changed
			? [
					{ value: 'text' as Mode, label: 'Text' },
					{ value: 'compare' as Mode, label: 'Compare' },
				]
			: []
	})
</script>

<div class="bar doc-toolbar">
	<span class="crumbs">
		{#each crumbs as crumb, i (i)}
			{#if i > 0}<span class="sep">/</span>{/if}
			<span class:doc-title={i === crumbs.length - 1}>{crumb}</span>
		{/each}
	</span>
	{#if dirty}<span class="tag">unsaved</span>{/if}
	<span class="spacer"></span>
	{#if modeOptions.length > 1}
		<Segmented
			options={modeOptions}
			value={mode}
			onchange={(next) => {
				mode = next
				fileView.remember(kind, next)
			}}
		/>
	{/if}
	{#if kind === 'image'}
		<span class="small mono">{imageSize}</span>
		<Segmented
			options={[
				{ value: 'fit', label: 'Fit' },
				{ value: '1', label: '100%' },
				{ value: '2', label: '200%' },
			]}
			bind:value={zoom}
		/>
		{#if rel.toLowerCase().endsWith('.png') && rel !== 'overrides/icon.png'}
			<button class="btn" onclick={useAsIcon}>Set as icon</button>
		{/if}
	{:else if kind === 'code' && lang !== 'text' && buffer?.state === 'text'}
		<button class="btn" onclick={format}>Format</button>
	{/if}
	{#if buffer?.state === 'text'}
		<button
			class="btn"
			class:primary={dirty}
			disabled={!dirty}
			onclick={pending(() => files.save(rel))}
			>Save <span class="keys">{keysFor('file.save') ?? ''}</span></button
		>
	{/if}
</div>

{#if failed}
	<Empty title="Can't open this file" detail={failed} />
{:else if kind === 'image'}
	{#if image}
		<ImageView
			src={image}
			zoom={zoom === 'fit' ? 'fit' : Number(zoom)}
			onmeasure={(w, h) => (imageSize = `${w} × ${h}`)}
		/>
	{/if}
{:else if kind === 'nbt'}
	{#if nbt}<NbtView root={nbt} />{/if}
{:else if !buffer || buffer.state === 'loading'}
	<div class="loading"><span class="spinner"></span></div>
{:else if buffer.state === 'missing'}
	<Empty title="File not found">
		<button class="btn primary" onclick={pending(() => createMissingFile(rel))}>Create</button>
		<button class="btn" onclick={() => tabs.close(doc.id)}>Close</button>
	</Empty>
{:else if buffer.state === 'binary' || buffer.state === 'large'}
	<Empty
		title={buffer.state === 'large'
			? `Too large to show (${bytes(buffer.size)})`
			: "Can't show this file"}
	>
		<button class="btn primary" onclick={() => ws.dir && void openInDefaultApp(join(ws.dir, rel))}
			>Open in default app</button
		>
		<button class="btn" onclick={() => ws.dir && void reveal(join(ws.dir, rel))}
			>Show in {fileManager}</button
		>
	</Empty>
{:else if mode === 'compare'}
	<div class="scroll"><DiffView {diff} split /></div>
{:else if mode === 'form'}
	<ConfigForm
		text={buffer.text}
		{lang}
		{baseline}
		{revealKey}
		keepPath={rel.startsWith('overrides/') ? rel.slice('overrides/'.length) : null}
		onchange={edit}
	/>
{:else if mode === 'preview'}
	<div class="scroll markdown-page"><Markdown source={buffer.text} /></div>
{:else if mode === 'split'}
	<div class="split">
		<CodeEditor value={buffer.text} {lang} base={baseline} {revealLine} onchange={edit} />
		<div class="scroll markdown-page"><Markdown source={buffer.text} /></div>
	</div>
{:else}
	<CodeEditor
		value={buffer.text}
		lang={kind === 'markdown' ? 'text' : lang}
		base={baseline}
		{hints}
		{revealLine}
		onchange={edit}
	/>
{/if}

<style>
	.crumbs {
		display: flex;
		align-items: center;
		gap: 6px;
		min-width: 0;
		overflow: hidden;
	}

	.loading {
		flex: 1;
		display: grid;
		place-items: center;
	}

	.split {
		flex: 1;
		min-height: 0;
		display: flex;
	}

	.split > :global(*) {
		flex: 1 1 50%;
		min-width: 0;
	}

	.split > :global(* + *) {
		border-left: 1px solid var(--line);
	}

	.markdown-page {
		padding: 20px 24px;
	}
</style>

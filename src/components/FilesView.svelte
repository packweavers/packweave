<script lang="ts">
	import { onMount } from 'svelte'
	import {
		Folder,
		FolderOpen,
		FileText,
		ChevronRight,
		CornerLeftUp,
		FolderPlus,
		FilePlus,
		Trash2,
		Pencil,
		Save,
		Search,
		ChevronUp,
		ChevronDown,
		X,
		Braces,
		SlidersHorizontal,
	} from '@lucide/svelte'
	import ButtonStyled from './ui/ButtonStyled.svelte'
	import CodeEditor from './ui/CodeEditor.svelte'
	import MarkdownEditor from './ui/MarkdownEditor.svelte'
	import ImageViewer from './ui/ImageViewer.svelte'
	import NbtViewer from './ui/NbtViewer.svelte'
	import { api } from '../api'
	import { store } from '../lib/store.svelte'
	import { contextMenu } from '../lib/contextmenu.svelte'
	import { tooltip } from '../lib/tooltip'
	import { fileFind } from '../lib/filefind.svelte'
	import type { FileContent, FsEntry, NbtNode } from '../types'
	import { langFromName } from '../highlight'
	import ConfigEditor, { isConfigForm } from './ui/ConfigEditor.svelte'
	import { formatBytes } from '../util'

	let { initialRoot, initialFile }: { initialRoot?: 'pack' | 'instance'; initialFile?: string } =
		$props()

	let root = $state<'pack' | 'instance'>('pack')
	let cwd = $state('')
	let entries = $state<FsEntry[]>([])
	let selected = $state<string | null>(null)
	let content = $state<FileContent | null>(null)
	let imageUrl = $state<string | null>(null)
	let nbt = $state<NbtNode | null>(null)
	let editText = $state('')

	let findOpen = $state(false)
	let findQuery = $state('')
	let findActive = $state(0)
	let nbtCount = $state(0)
	let editorEl = $state<HTMLTextAreaElement | null>(null)
	let findInputEl = $state<HTMLInputElement | null>(null)
	const fbBtn =
		'grid place-items-center w-6 h-6 rounded-sm text-secondary bg-transparent border-none cursor-pointer enabled:hover:bg-button-bg enabled:hover:text-contrast disabled:opacity-40 disabled:cursor-default'

	const IMAGE_EXTS = ['png', 'jpg', 'jpeg', 'gif', 'webp', 'svg', 'bmp', 'ico']
	const isImageName = (n: string) => IMAGE_EXTS.includes(n.split('.').pop()?.toLowerCase() ?? '')
	const isMarkdownName = (n: string) =>
		['md', 'markdown', 'mdx'].includes(n.split('.').pop()?.toLowerCase() ?? '')
	const isNbtName = (n: string) =>
		['dat', 'dat_old', 'nbt', 'schematic', 'litematic'].includes(
			n.split('.').pop()?.toLowerCase() ?? '',
		)
	let dirty = $state(false)
	let creating = $state<'folder' | 'file' | null>(null)
	let newName = $state('')
	let renaming = $state<string | null>(null)
	let renameName = $state('')
	let confirmDelete = $state<string | null>(null)

	const rootPath = $derived(root === 'pack' ? (store.pack?.dir ?? '') : (store.instanceDir ?? ''))
	const crumbs = $derived(cwd ? cwd.split('/') : [])
	const lang = $derived(selected ? langFromName(selected) : 'text')
	const canFormat = $derived(content?.text != null && lang !== 'text')
	const configCapable = $derived(!!selected && content?.text != null && isConfigForm(selected))
	let configMode = $state(false)
	let codeRef = $state<{ format: () => void }>()
	const lintError = $derived.by(() => {
		if (!content || content.text === null || lang !== 'json') return null
		try {
			JSON.parse(editText)
			return null
		} catch (e) {
			return (e as Error).message
		}
	})

	$effect(() => {
		if (content && content.text !== null && editText !== content.text) dirty = true
	})

	const textMatches = $derived.by<Array<[number, number]>>(() => {
		if (!content || content.text === null || !findQuery) return []
		const q = findQuery.toLowerCase()
		const hay = editText.toLowerCase()
		const out: Array<[number, number]> = []
		let i = hay.indexOf(q)
		while (i !== -1 && out.length < 5000) {
			out.push([i, i + q.length])
			i = hay.indexOf(q, i + q.length)
		}
		return out
	})
	const findCount = $derived(nbt ? nbtCount : textMatches.length)

	function applyText() {
		if (nbt) return
		const m = textMatches[findActive]
		if (m && editorEl) {
			editorEl.setSelectionRange(m[0], m[1])
			const line = editText.slice(0, m[0]).split('\n').length - 1
			const lh = parseFloat(getComputedStyle(editorEl).lineHeight) || 18
			editorEl.scrollTop = Math.max(0, line * lh - editorEl.clientHeight / 2)
		}
	}
	function findNext() {
		if (!findCount) return
		findActive = (findActive + 1) % findCount
		applyText()
	}
	function findPrev() {
		if (!findCount) return
		findActive = (findActive - 1 + findCount) % findCount
		applyText()
	}
	function onFindInput() {
		findActive = 0
		applyText()
	}
	function openFind(): boolean {
		if (!selected || (content?.text == null && !nbt)) return false
		findOpen = true
		findActive = 0
		requestAnimationFrame(() => findInputEl?.select())
		return true
	}
	function closeFind() {
		findOpen = false
		editorEl?.focus()
	}
	$effect(() => {
		fileFind.open = openFind
		return () => {
			if (fileFind.open === openFind) fileFind.open = null
		}
	})

	const join = (base: string, name: string) => (base ? `${base}/${name}` : name)

	async function loadDir() {
		confirmDelete = null
		creating = null
		renaming = null
		try {
			entries = await api.fsList(rootPath, cwd)
		} catch (e) {
			entries = []
			store.notify('error', `${e}`)
		}
	}

	function switchRoot(r: 'pack' | 'instance') {
		if (root === r) return
		root = r
		cwd = ''
		selected = null
		content = null
		imageUrl = null
		nbt = null
		loadDir()
	}

	async function open(entry: FsEntry) {
		findOpen = false
		if (entry.isDir) {
			cwd = join(cwd, entry.name)
			selected = null
			content = null
			imageUrl = null
			nbt = null
			await loadDir()
		} else {
			const path = join(cwd, entry.name)
			try {
				nbt = null
				if (isImageName(entry.name)) {
					imageUrl = await api.fsReadImage(rootPath, path)
					content = null
					selected = path
					return
				}
				if (isNbtName(entry.name)) {
					try {
						nbt = await api.fsReadNbt(rootPath, path)
						content = null
						imageUrl = null
						selected = path
						return
					} catch {
						nbt = null
					}
				}
				imageUrl = null
				content = await api.fsRead(rootPath, path)
				selected = path
				editText = content.text ?? ''
				configMode = content.text != null && isConfigForm(path)
				dirty = false
			} catch (e) {
				store.notify('error', `${e}`)
			}
		}
	}

	function goUp() {
		const parts = cwd.split('/').filter(Boolean)
		parts.pop()
		cwd = parts.join('/')
		selected = null
		content = null
		imageUrl = null
		nbt = null
		loadDir()
	}

	function goCrumb(i: number) {
		cwd = crumbs.slice(0, i + 1).join('/')
		selected = null
		content = null
		imageUrl = null
		nbt = null
		loadDir()
	}

	async function save() {
		if (!selected) return
		const rel = selected
		const onPack = root === 'pack'
		const original = content?.text ?? null
		try {
			await api.fsWrite(rootPath, rel, editText)
			dirty = false
			if (content) content.text = editText
			let pushed = false
			if (onPack && store.autoPushOnSave && store.instanceDir && store.pack) {
				try {
					pushed = await api.autoPushFile(store.pack.dir, rel, original)
				} catch {
				}
			}
			store.notify('success', pushed ? 'Saved · pushed to instance' : 'Saved')
			if (onPack) await store.refreshGit()
			await store.refreshSync()
		} catch (e) {
			store.notify('error', `${e}`)
		}
	}

	function onSaveKey(e: KeyboardEvent) {
		if ((e.metaKey || e.ctrlKey) && (e.key === 's' || e.key === 'S')) {
			if (selected && content?.text != null) {
				e.preventDefault()
				if (dirty) save()
			}
		} else if (e.shiftKey && (e.altKey || e.metaKey || e.ctrlKey) && e.code === 'KeyF') {
			if (canFormat && !configMode) {
				e.preventDefault()
				codeRef?.format()
			}
		}
	}

	async function commitCreate() {
		const name = newName.trim()
		if (!name) {
			creating = null
			return
		}
		try {
			if (creating === 'folder') {
				await api.fsMkdir(rootPath, join(cwd, name))
			} else {
				await api.fsWrite(rootPath, join(cwd, name), '')
			}
			newName = ''
			creating = null
			await loadDir()
		} catch (e) {
			store.notify('error', `${e}`)
		}
	}

	function startCreate(kind: 'folder' | 'file') {
		creating = kind
		newName = ''
	}

	function startRename(entry: FsEntry) {
		confirmDelete = null
		creating = null
		renaming = entry.name
		renameName = entry.name
	}

	async function commitRename(from: string) {
		const name = renameName.trim()
		renaming = null
		if (!name || name === from) return
		try {
			await api.fsRename(rootPath, join(cwd, from), join(cwd, name))
			if (selected === join(cwd, from)) selected = join(cwd, name)
			await loadDir()
		} catch (e) {
			store.notify('error', `${e}`)
		}
	}

	async function doDelete(entry: FsEntry) {
		try {
			await api.fsDelete(rootPath, join(cwd, entry.name))
			if (selected === join(cwd, entry.name)) {
				selected = null
				content = null
			}
			await loadDir()
		} catch (e) {
			store.notify('error', `${e}`)
		}
	}

	onMount(async () => {
		if (initialFile) {
			root = initialRoot ?? 'pack'
			const parts = initialFile.split('/')
			const name = parts.pop() ?? ''
			cwd = parts.join('/')
			await loadDir()
			const entry = entries.find((e) => e.name === name)
			if (entry) await open(entry)
		} else {
			await loadDir()
		}
	})
</script>

<svelte:window onkeydown={onSaveKey} />

<div class="h-full flex flex-col overflow-hidden">
	<header class="flex items-center justify-between px-4 py-[0.7rem] border-b border-divider">
		<div class="flex gap-1">
			<button
				onclick={() => switchRoot('pack')}
				class={`bg-transparent border-none text-[0.85rem] font-semibold px-[0.7rem] py-[0.3rem] rounded-sm cursor-pointer ${
					root === 'pack' ? 'bg-button-bg text-contrast' : 'text-secondary'
				}`}
			>
				Pack
			</button>
			<button
				disabled={!store.instanceDir}
				onclick={() => switchRoot('instance')}
				class={`bg-transparent border-none text-[0.85rem] font-semibold px-[0.7rem] py-[0.3rem] rounded-sm cursor-pointer disabled:opacity-40 disabled:cursor-default ${
					root === 'instance' ? 'bg-button-bg text-contrast' : 'text-secondary'
				}`}
			>
				Instance
			</button>
		</div>
	</header>

	<div class="flex items-center gap-[0.1rem] px-4 py-2 border-b border-divider flex-wrap">
		<button
			onclick={() => goCrumb(-1)}
			class="bg-transparent border-none text-secondary text-[0.76rem] cursor-pointer px-1 py-[0.1rem] rounded-sm hover:text-body hover:bg-button-bg"
		>
			{root === 'pack' ? 'pack' : 'instance'}
		</button>
		{#each crumbs as c, i (i)}
			<ChevronRight size={13} class="text-secondary" />
			<button
				onclick={() => goCrumb(i)}
				class="bg-transparent border-none text-secondary text-[0.76rem] cursor-pointer px-1 py-[0.1rem] rounded-sm hover:text-body hover:bg-button-bg"
			>
				{c}
			</button>
		{/each}
	</div>

	<div class="flex-1 grid grid-cols-[300px_1fr] min-h-0">
		<div class="flex flex-col border-r border-divider min-h-0">
			<div class="flex gap-[0.3rem] p-2 border-b border-divider">
				<button
					onclick={() => startCreate('folder')}
					use:tooltip={'New folder'}
					aria-label="New folder"
					class="inline-flex items-center gap-1 bg-transparent border-none text-secondary text-[0.74rem] cursor-pointer px-[0.35rem] py-[0.2rem] rounded-sm hover:bg-button-bg hover:text-body"
				>
					<FolderPlus size={14} /> Folder
				</button>
				<button
					onclick={() => startCreate('file')}
					use:tooltip={'New file'}
					aria-label="New file"
					class="inline-flex items-center gap-1 bg-transparent border-none text-secondary text-[0.74rem] cursor-pointer px-[0.35rem] py-[0.2rem] rounded-sm hover:bg-button-bg hover:text-body"
				>
					<FilePlus size={14} /> File
				</button>
			</div>
			<ul class="list-none m-0 p-[0.3rem] overflow-y-auto flex-1">
				{#if cwd}
					<li
						onclick={goUp}
						use:tooltip={'Go up'}
						aria-label="Go up"
						class="flex items-center gap-2 px-[0.45rem] py-[0.35rem] rounded-sm cursor-pointer text-[0.82rem] text-body hover:bg-bg-raised"
					>
						<CornerLeftUp size={15} /><span>..</span>
					</li>
				{/if}
				{#if creating}
					<li class="flex items-center gap-2 px-[0.45rem] py-[0.35rem] rounded-sm text-[0.82rem] text-body">
						{#if creating === 'folder'}
							<Folder size={15} />
						{:else}
							<FileText size={15} />
						{/if}
						<!-- svelte-ignore a11y_autofocus -->
						<input
							bind:value={newName}
							placeholder={creating === 'folder' ? 'folder name' : 'file name'}
							autofocus
							onkeydown={(e) => {
								if (e.key === 'Enter') commitCreate()
								else if (e.key === 'Escape') creating = null
							}}
							onblur={commitCreate}
							class="flex-1 bg-bg border border-brand text-contrast rounded-sm px-[0.35rem] py-[0.15rem] text-[0.8rem] outline-none"
						/>
					</li>
				{/if}
				{#each entries as entry (entry.name)}
					{#if renaming === entry.name}
						<li class="flex items-center gap-2 px-[0.45rem] py-[0.35rem] rounded-sm text-[0.82rem] text-body">
							{#if entry.isDir}
								<Folder size={15} class="text-blue" />
							{:else}
								<FileText size={15} class="text-secondary" />
							{/if}
							<!-- svelte-ignore a11y_autofocus -->
							<input
								bind:value={renameName}
								autofocus
								onkeydown={(e) => {
									if (e.key === 'Enter') commitRename(entry.name)
									else if (e.key === 'Escape') renaming = null
								}}
								onblur={() => commitRename(entry.name)}
								class="flex-1 bg-bg border border-brand text-contrast rounded-sm px-[0.35rem] py-[0.15rem] text-[0.8rem] outline-none"
							/>
						</li>
					{:else}
						<li
							onclick={() => open(entry)}
							use:contextMenu={() => [
								{
									label: 'Open',
									icon: entry.isDir ? FolderOpen : FileText,
									onSelect: () => open(entry),
								},
								{ label: 'Rename', icon: Pencil, onSelect: () => startRename(entry) },
								{ separator: true },
								{ label: 'Delete', icon: Trash2, danger: true, onSelect: () => doDelete(entry) },
							]}
							class={`group flex items-center gap-2 px-[0.45rem] py-[0.35rem] rounded-sm cursor-pointer text-[0.82rem] ${
								selected === join(cwd, entry.name)
									? 'bg-brand/15 text-brand'
									: 'text-body hover:bg-bg-raised'
							}`}
						>
							{#if entry.isDir}
								<Folder size={15} class="text-blue" />
							{:else}
								<FileText size={15} class="text-secondary" />
							{/if}
							<span class="flex-1 whitespace-nowrap overflow-hidden text-ellipsis">{entry.name}</span>
							{#if !entry.isDir}
								<span class="text-[0.68rem] text-secondary">{formatBytes(entry.size)}</span>
							{/if}
							{#if confirmDelete !== entry.name}
								<button
									onclick={(e) => {
										e.stopPropagation()
										confirmDelete = entry.name
									}}
									use:tooltip={'Delete'}
									aria-label="Delete"
									class="bg-transparent border-none text-secondary cursor-pointer opacity-0 grid place-items-center group-hover:opacity-100 hover:text-red"
								>
									<Trash2 size={13} />
								</button>
							{:else}
								<span
									onclick={(e) => e.stopPropagation()}
									role="group"
									class="inline-flex items-center gap-1"
								>
									<button
										onclick={() => doDelete(entry)}
										class="bg-transparent border-none text-red text-[0.74rem] font-semibold cursor-pointer px-1 rounded-sm hover:bg-red/15"
									>
										Delete
									</button>
									<button
										onclick={() => (confirmDelete = null)}
										class="bg-transparent border-none text-secondary text-[0.74rem] cursor-pointer px-1 rounded-sm hover:bg-button-bg"
									>
										No
									</button>
								</span>
							{/if}
						</li>
					{/if}
				{/each}
			</ul>
		</div>

		<div class="relative flex flex-col min-h-0 min-w-0">
			{#if findOpen && (content?.text != null || nbt)}
				<div
					class="absolute top-[2.9rem] right-3 z-20 flex items-center gap-1 bg-bg-super-raised border border-divider rounded-md shadow-floating px-1.5 py-1"
				>
					<Search size={13} class="text-secondary shrink-0" />
					<input
						bind:this={findInputEl}
						bind:value={findQuery}
						oninput={onFindInput}
						onkeydown={(e) => {
							if (e.key === 'Enter') {
								e.preventDefault()
								e.shiftKey ? findPrev() : findNext()
							} else if (e.key === 'Escape') {
								e.preventDefault()
								closeFind()
							}
						}}
						placeholder="Find in file"
						spellcheck="false"
						class="w-40 bg-transparent border-0 outline-none text-[0.8rem] text-contrast"
					/>
					<span class="text-[0.7rem] text-secondary tabular-nums shrink-0 min-w-[2.6rem] text-center">
						{findQuery ? `${findCount ? findActive + 1 : 0}/${findCount}` : ''}
					</span>
					<button
						use:tooltip={'Previous  ⇧⏎'}
						aria-label="Previous match"
						disabled={!findCount}
						onclick={findPrev}
						class={fbBtn}><ChevronUp size={14} /></button
					>
					<button
						use:tooltip={'Next  ⏎'}
						aria-label="Next match"
						disabled={!findCount}
						onclick={findNext}
						class={fbBtn}><ChevronDown size={14} /></button
					>
					<button use:tooltip={'Close  Esc'} aria-label="Close find" onclick={closeFind} class={fbBtn}
						><X size={14} /></button
					>
				</div>
			{/if}
			{#if selected && imageUrl}
				<div class="flex items-center justify-between gap-2 px-[0.7rem] py-2 border-b border-divider">
					<span
						class="flex-1 min-w-0 text-[0.76rem] text-secondary font-mono whitespace-nowrap overflow-hidden text-ellipsis"
						>{selected}</span
					>
				</div>
				<ImageViewer src={imageUrl} />
			{:else if selected && nbt}
				<div class="flex items-center justify-between gap-2 px-[0.7rem] py-2 border-b border-divider">
					<span
						class="flex-1 min-w-0 text-[0.76rem] text-secondary font-mono whitespace-nowrap overflow-hidden text-ellipsis"
						>{selected}</span
					>
					<span class="text-[0.7rem] text-secondary shrink-0">NBT · read-only</span>
				</div>
				<NbtViewer
					root={nbt}
					query={findOpen ? findQuery : ''}
					active={findActive}
					bind:matchCount={nbtCount}
				/>
			{:else if selected && content}
				<div class="flex items-center justify-between gap-2 px-[0.7rem] py-2 border-b border-divider">
					<span
						class="flex-1 min-w-0 text-[0.76rem] text-secondary font-mono whitespace-nowrap overflow-hidden text-ellipsis"
						>{selected}</span
					>
					<div class="flex items-center gap-2 shrink-0">
						{#if lintError && !configMode}
							<span
								title={lintError}
								class="text-[0.72rem] font-semibold text-red bg-red/15 px-[0.45rem] py-[0.15rem] rounded-sm whitespace-nowrap"
								>Invalid JSON</span
							>
						{/if}
						{#if configCapable}
							<div class="inline-flex bg-bg-inset border border-divider rounded-sm p-[2px] gap-[2px]">
								<button
									class={`inline-flex items-center gap-[0.25rem] border-none text-[0.72rem] font-[550] px-[0.45rem] py-[0.2rem] rounded-[3px] cursor-pointer ${
										configMode ? 'bg-transparent text-secondary hover:text-body' : 'bg-bg-raised text-contrast'
									}`}
									onclick={() => (configMode = false)}>Text</button
								>
								<button
									class={`inline-flex items-center gap-[0.25rem] border-none text-[0.72rem] font-[550] px-[0.45rem] py-[0.2rem] rounded-[3px] cursor-pointer ${
										configMode ? 'bg-bg-raised text-contrast' : 'bg-transparent text-secondary hover:text-body'
									}`}
									onclick={() => (configMode = true)}><SlidersHorizontal size={12} /> Form</button
								>
							</div>
						{/if}
						{#if canFormat && !configMode}
							<button
								use:tooltip={'Format  ⌥⇧F'}
								aria-label="Format"
								onclick={() => codeRef?.format()}
								class={fbBtn}
							>
								<Braces size={15} />
							</button>
						{/if}
						{#if content.text !== null}
							<ButtonStyled size="small" color="brand" disabled={!dirty} onclick={save}>
								<Save size={14} /> Save
							</ButtonStyled>
						{/if}
					</div>
				</div>
				{#if content.text !== null}
					{#if selected && isMarkdownName(selected)}
						<div class="flex-1 min-h-0 p-2 flex flex-col">
							<MarkdownEditor bind:value={editText} bind:element={editorEl} />
						</div>
					{:else if configMode && configCapable}
						<ConfigEditor
							bind:value={editText}
							{lang}
							onsave={() => {
								if (dirty) save()
							}}
						/>
					{:else}
						<CodeEditor
							bind:this={codeRef}
							bind:value={editText}
							{lang}
							bind:element={editorEl}
							onsave={() => {
								if (dirty) save()
							}}
						/>
					{/if}
				{:else}
					<div
						class="flex-1 grid place-items-center text-secondary text-center p-8 text-[0.85rem]"
					>
						{content.binary ? 'Binary file. Cannot edit here.' : 'File is too large to edit.'}
						<div class="text-secondary opacity-70 text-[0.75rem] mt-[0.3rem]">
							{formatBytes(content.size)}
						</div>
					</div>
				{/if}
			{:else}
				<div class="flex-1 grid place-items-center text-secondary text-center p-8 text-[0.85rem]">
					Select a file to view or edit it.
				</div>
			{/if}
		</div>
	</div>
</div>

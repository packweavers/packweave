import { invoke } from '@tauri-apps/api/core'
import { open, save } from '@tauri-apps/plugin-dialog'
import { openPath, openUrl } from '@tauri-apps/plugin-opener'
import type {
	Branches,
	BulkCandidate,
	BulkLookup,
	DroppedFile,
	DetectedInstance,
	DownloadReport,
	FileContent,
	NbtNode,
	FileMatch,
	FsEntry,
	ConvertCandidate,
	GitChange,
	GitCommit,
	GitStatus,
	LoaderVersions,
	LocalContent,
	Remote,
	Stash,
	Tag,
	ImpactReport,
	PackDiff,
	Manifest,
	ModMeta,
	ProviderInfo,
	VersionInfo,
	PackEnv,
	PackResolved,
	PackState,
	PullStrategy,
	SearchHit,
	SyncOp,
	SyncReport,
} from './types'

export const api = {
	createPack(
		path: string,
		name: string,
		minecraft: string,
		loader: string,
		loaderVersion: string | null,
	): Promise<Manifest> {
		return invoke('create_pack', { path, name, minecraft, loader, loaderVersion })
	},

	openPack(path: string): Promise<PackState> {
		return invoke('open_pack', { path })
	},

	importPack(src: string, dest: string): Promise<PackState> {
		return invoke('import_pack', { src, dest })
	},

	clonePack(url: string, destParent: string): Promise<PackState> {
		return invoke('clone_pack', { url, destParent })
	},

	saveManifest(path: string, manifest: Manifest): Promise<void> {
		return invoke('save_manifest', { path, manifest })
	},

	search(
		provider: string,
		query: string,
		minecraft: string,
		loader: string,
		projectType: string,
		offset: number,
		limit: number,
	): Promise<SearchHit[]> {
		return invoke('search', { provider, query, minecraft, loader, projectType, offset, limit })
	},

	listProviders(): Promise<ProviderInfo[]> {
		return invoke('list_providers')
	},

	addContent(
		path: string,
		provider: string,
		projectId: string,
		slug: string,
		name: string,
		projectType: string,
		pin: string | null,
		url: string | null,
	): Promise<PackResolved> {
		return invoke('add_content', { path, provider, projectId, slug, name, projectType, pin, url })
	},

	bulkLookup(path: string, provider: string, text: string): Promise<BulkLookup> {
		return invoke('bulk_lookup', { path, provider, text })
	},

	addContentBulk(path: string, items: BulkCandidate[]): Promise<PackResolved> {
		return invoke('add_content_bulk', { path, items })
	},

	identifyDropped(path: string, files: string[]): Promise<DroppedFile[]> {
		return invoke('identify_dropped', { path, files })
	},

	addDropped(path: string, items: DroppedFile[]): Promise<PackResolved> {
		return invoke('add_dropped', { path, items })
	},

	addMod(
		path: string,
		projectId: string,
		slug: string,
		name: string,
		projectType: string,
	): Promise<PackResolved> {
		return invoke('add_mod', { path, projectId, slug, name, projectType })
	},

	removeMod(path: string, projectId: string): Promise<PackResolved> {
		return invoke('remove_mod', { path, projectId })
	},

	deleteContent(path: string, keys: string[]): Promise<PackResolved> {
		return invoke('delete_content', { path, keys })
	},

	setAsDependency(path: string, projectId: string): Promise<PackResolved> {
		return invoke('set_as_dependency', { path, projectId })
	},

	promoteMod(path: string, projectId: string): Promise<PackResolved> {
		return invoke('promote_mod', { path, projectId })
	},

	setContentDisabled(path: string, key: string, disabled: boolean): Promise<PackResolved> {
		return invoke('set_content_disabled', { path, key, disabled })
	},

	setContentDisabledBulk(path: string, keys: string[], disabled: boolean): Promise<PackResolved> {
		return invoke('set_content_disabled_bulk', { path, keys, disabled })
	},

	removeModsBulk(path: string, keys: string[]): Promise<PackResolved> {
		return invoke('remove_mods_bulk', { path, keys })
	},

	convertSearch(path: string, projectId: string, target: string): Promise<ConvertCandidate> {
		return invoke('convert_search', { path, projectId, target })
	},

	convertLookup(
		path: string,
		projectId: string,
		target: string,
		query: string,
	): Promise<ConvertCandidate> {
		return invoke('convert_lookup', { path, projectId, target, query })
	},

	addAltSource(
		path: string,
		projectId: string,
		provider: string,
		id: string,
	): Promise<PackResolved> {
		return invoke('add_alt_source', { path, projectId, provider, id })
	},

	setPreferredSource(path: string, projectId: string, provider: string): Promise<PackResolved> {
		return invoke('set_preferred_source', { path, projectId, provider })
	},

	removeAltSource(path: string, projectId: string, provider: string): Promise<PackResolved> {
		return invoke('remove_alt_source', { path, projectId, provider })
	},

	enrichMods(items: { id: string; provider: string }[]): Promise<ModMeta[]> {
		return invoke('enrich_mods', { items })
	},

	modVersions(
		projectId: string,
		provider: string,
		minecraft: string,
		loader: string | null,
	): Promise<VersionInfo[]> {
		return invoke('mod_versions', { projectId, provider, minecraft, loader })
	},

	setModVersion(path: string, projectId: string, version: string): Promise<PackResolved> {
		return invoke('set_mod_version', { path, projectId, version })
	},

	setModVersions(
		path: string,
		updates: { projectId: string; version: string }[],
	): Promise<PackResolved> {
		return invoke('set_mod_versions', { path, updates })
	},

	updateImpact(
		path: string,
		updates: { projectId: string; version: string }[],
	): Promise<ImpactReport> {
		return invoke('update_impact', { path, updates })
	},

	resolvePack(path: string, force = false): Promise<PackResolved> {
		return invoke('resolve_pack', { path, force })
	},

	exportMrpack(path: string, output: string, env: PackEnv): Promise<void> {
		return invoke('export_mrpack', { path, output, env })
	},

	exportCurseforge(path: string, output: string, env: PackEnv): Promise<void> {
		return invoke('export_curseforge', { path, output, env })
	},

	exportInstance(path: string, output: string, env: PackEnv): Promise<void> {
		return invoke('export_instance', { path, output, env })
	},

	exportMrpackSelfUpdate(path: string, output: string, url: string): Promise<void> {
		return invoke('export_mrpack_selfupdate', { path, output, url })
	},

	exportCurseforgeSelfUpdate(path: string, output: string, url: string): Promise<void> {
		return invoke('export_curseforge_selfupdate', { path, output, url })
	},

	bindInstance(path: string, instance: string): Promise<void> {
		return invoke('bind_instance', { path, instance })
	},

	unbindInstance(path: string): Promise<void> {
		return invoke('unbind_instance', { path })
	},

	getBinding(path: string): Promise<string | null> {
		return invoke('get_binding', { path })
	},

	syncStatus(path: string): Promise<SyncReport> {
		return invoke('sync_status', { path })
	},

	applySync(path: string, ops: SyncOp[]): Promise<PackResolved> {
		return invoke('apply_sync', { path, ops })
	},

	downloadMods(path: string, output: string): Promise<DownloadReport> {
		return invoke('download_mods', { path, output })
	},

	getLoaderVersions(loader: string, minecraft: string): Promise<LoaderVersions> {
		return invoke('get_loader_versions', { loader, minecraft })
	},

	getMinecraftVersions(includeSnapshots = false): Promise<string[]> {
		return invoke('get_minecraft_versions', { includeSnapshots })
	},

	syncFileDiff(path: string, rel: string, kind: string): Promise<string> {
		return invoke('sync_file_diff', { path, rel, kind })
	},

	autoPushFile(path: string, rel: string, original: string | null): Promise<boolean> {
		return invoke('auto_push_file', { path, rel, original })
	},

	gitStatus(path: string): Promise<GitStatus> {
		return invoke('git_status', { path })
	},

	gitInit(path: string): Promise<void> {
		return invoke('git_init', { path })
	},

	gitCommit(path: string, message: string, files: string[], amend = false): Promise<void> {
		return invoke('git_commit', { path, message, files, amend })
	},

	gitLog(path: string, limit: number, refname?: string, file?: string): Promise<GitCommit[]> {
		return invoke('git_log', { path, limit, refname: refname ?? null, file: file ?? null })
	},

	gitPush(path: string, force = false, tags = false): Promise<string> {
		return invoke('git_push', { path, force, tags })
	},

	gitPull(path: string, strategy: PullStrategy = 'ff'): Promise<string> {
		return invoke('git_pull', { path, strategy })
	},

	gitDiscard(path: string): Promise<void> {
		return invoke('git_discard', { path })
	},

	readGitignore(path: string): Promise<string> {
		return invoke('read_gitignore', { path })
	},

	writeGitignore(path: string, content: string): Promise<void> {
		return invoke('write_gitignore', { path, content })
	},

	detectInstances(): Promise<DetectedInstance[]> {
		return invoke('detect_instances')
	},

	resolveInstanceFolder(path: string): Promise<DetectedInstance> {
		return invoke('resolve_instance_folder', { path })
	},

	checkUpdate(): Promise<{ version: string; notes: string } | null> {
		return invoke('check_update')
	},
	installUpdate(): Promise<void> {
		return invoke('install_update')
	},

	packIcon(path: string): Promise<string | null> {
		return invoke('pack_icon', { path })
	},
	setPackIcon(path: string, source: string): Promise<void> {
		return invoke('set_pack_icon', { path, source })
	},
	clearPackIcon(path: string): Promise<void> {
		return invoke('clear_pack_icon', { path })
	},

	fsList(root: string, rel: string): Promise<FsEntry[]> {
		return invoke('fs_list', { root, rel })
	},

	fsRead(root: string, rel: string): Promise<FileContent> {
		return invoke('fs_read', { root, rel })
	},

	fsWrite(root: string, rel: string, content: string): Promise<void> {
		return invoke('fs_write', { root, rel, content })
	},

	fsMkdir(root: string, rel: string): Promise<void> {
		return invoke('fs_mkdir', { root, rel })
	},

	fsDelete(root: string, rel: string): Promise<void> {
		return invoke('fs_delete', { root, rel })
	},

	fsRename(root: string, from: string, to: string): Promise<void> {
		return invoke('fs_rename', { root, from, to })
	},

	fsReadImage(root: string, rel: string): Promise<string> {
		return invoke('fs_read_image', { root, rel })
	},

	fsReadNbt(root: string, rel: string): Promise<NbtNode> {
		return invoke('fs_read_nbt', { root, rel })
	},

	searchFiles(root: string, query: string): Promise<FileMatch[]> {
		return invoke('search_files', { root, query })
	},

	gitDiffFile(path: string, file: string, staged: boolean): Promise<string> {
		return invoke('git_diff_file', { path, file, staged })
	},

	gitDiscardFile(path: string, file: string): Promise<void> {
		return invoke('git_discard_file', { path, file })
	},

	gitRevert(path: string, files: string[]): Promise<void> {
		return invoke('git_revert', { path, files })
	},

	gitResolveConflict(path: string, file: string, side: 'ours' | 'theirs'): Promise<void> {
		return invoke('git_resolve_conflict', { path, file, side })
	},

	gitBranches(path: string): Promise<Branches> {
		return invoke('git_branches', { path })
	},

	gitCheckout(path: string, branch: string): Promise<void> {
		return invoke('git_checkout', { path, branch })
	},

	gitCreateBranch(path: string, name: string, startPoint?: string, checkout = true): Promise<void> {
		return invoke('git_create_branch', { path, name, startPoint: startPoint ?? null, checkout })
	},

	gitRenameBranch(path: string, oldName: string, newName: string): Promise<void> {
		return invoke('git_rename_branch', { path, old: oldName, new: newName })
	},

	gitDeleteBranch(path: string, name: string, force = false): Promise<void> {
		return invoke('git_delete_branch', { path, name, force })
	},

	gitDeleteRemoteBranch(path: string, name: string): Promise<string> {
		return invoke('git_delete_remote_branch', { path, name })
	},

	gitMerge(path: string, name: string): Promise<string> {
		return invoke('git_merge', { path, name })
	},

	gitRebase(path: string, name: string): Promise<string> {
		return invoke('git_rebase', { path, name })
	},

	gitSetUpstream(path: string, upstream: string): Promise<void> {
		return invoke('git_set_upstream', { path, upstream })
	},

	gitFetch(path: string): Promise<string> {
		return invoke('git_fetch', { path })
	},

	gitPushBranch(path: string, remote: string, branch: string, setUpstream = true): Promise<string> {
		return invoke('git_push_branch', { path, remote, branch, setUpstream })
	},

	gitCommitChanges(path: string, hash: string): Promise<GitChange[]> {
		return invoke('git_commit_changes', { path, hash })
	},

	gitShowDiff(path: string, hash: string, file: string): Promise<string> {
		return invoke('git_show_diff', { path, hash, file })
	},

	gitPackDiff(path: string, from: string, to: string): Promise<PackDiff> {
		return invoke('git_pack_diff', { path, from, to })
	},

	gitPackDiffWorking(path: string): Promise<PackDiff> {
		return invoke('git_pack_diff_working', { path })
	},

	changelogBetween(
		path: string,
		from: string,
		to: string,
		heading?: string,
		links = false,
		forFile = false,
	): Promise<string> {
		return invoke('changelog_between', { path, from, to, heading: heading ?? null, links, forFile })
	},

	changelogWorking(
		path: string,
		heading?: string,
		links = false,
		forFile = false,
	): Promise<string> {
		return invoke('changelog_working', { path, heading: heading ?? null, links, forFile })
	},

	changelogSave(path: string, section: string): Promise<void> {
		return invoke('changelog_save', { path, section })
	},

	changelogHead(path: string): Promise<string | null> {
		return invoke('changelog_head', { path })
	},

	gitLatestTag(path: string): Promise<string | null> {
		return invoke('git_latest_tag', { path })
	},

	gitRevertCommit(path: string, hash: string): Promise<string> {
		return invoke('git_revert_commit', { path, hash })
	},

	gitReset(path: string, hash: string, mode: 'soft' | 'mixed' | 'hard'): Promise<void> {
		return invoke('git_reset', { path, hash, mode })
	},

	gitCherryPick(path: string, hash: string): Promise<string> {
		return invoke('git_cherry_pick', { path, hash })
	},

	gitStashList(path: string): Promise<Stash[]> {
		return invoke('git_stash_list', { path })
	},

	gitStashPush(path: string, message: string, includeUntracked: boolean): Promise<string> {
		return invoke('git_stash_push', { path, message, includeUntracked })
	},

	gitStashApply(path: string, reference: string, drop: boolean): Promise<string> {
		return invoke('git_stash_apply', { path, reference, drop })
	},

	gitStashDrop(path: string, reference: string): Promise<void> {
		return invoke('git_stash_drop', { path, reference })
	},

	gitTags(path: string): Promise<Tag[]> {
		return invoke('git_tags', { path })
	},

	gitCreateTag(path: string, name: string, message?: string, target?: string): Promise<void> {
		return invoke('git_create_tag', {
			path,
			name,
			message: message ?? null,
			target: target ?? null,
		})
	},

	gitDeleteTag(path: string, name: string): Promise<void> {
		return invoke('git_delete_tag', { path, name })
	},

	gitPushTag(path: string, name: string): Promise<string> {
		return invoke('git_push_tag', { path, name })
	},

	githubRelease(path: string, tag: string, name: string, body: string): Promise<string> {
		return invoke('github_release', { path, tag, name, body })
	},

	gitPackUrl(path: string): Promise<string | null> {
		return invoke('git_pack_url', { path })
	},

	gitRemotes(path: string): Promise<Remote[]> {
		return invoke('git_remotes', { path })
	},

	gitAddRemote(path: string, name: string, url: string): Promise<void> {
		return invoke('git_add_remote', { path, name, url })
	},

	gitSetRemoteUrl(path: string, name: string, url: string): Promise<void> {
		return invoke('git_set_remote_url', { path, name, url })
	},

	gitRemoveRemote(path: string, name: string): Promise<void> {
		return invoke('git_remove_remote', { path, name })
	},

	listUnpublished(path: string): Promise<LocalContent[]> {
		return invoke('list_unpublished', { path })
	},

	exportDist(path: string, output: string, packUrl: string): Promise<void> {
		return invoke('export_dist', { path, output, packUrl })
	},

	publishPack(
		target: string,
		path: string,
		projectId: string,
		versionNumber: string,
		changelog: string,
		env: string,
	): Promise<string> {
		return invoke('publish_pack', { target, path, projectId, versionNumber, changelog, env })
	},

	secretSet(key: string, value: string): Promise<void> {
		return invoke('secret_set', { key, value })
	},

	secretDelete(key: string): Promise<void> {
		return invoke('secret_delete', { key })
	},

	readPrefs(): Promise<string> {
		return invoke('read_prefs')
	},

	writePrefs(content: string): Promise<void> {
		return invoke('write_prefs', { content })
	},
}

export async function pickImportFile(): Promise<string | null> {
	const result = await open({
		multiple: false,
		title: 'Import a modpack',
		filters: [{ name: 'Modpack', extensions: ['mrpack', 'zip'] }],
	})
	if (Array.isArray(result)) return result[0] ?? null
	return result
}

export async function pickFolder(title: string): Promise<string | null> {
	const result = await open({ directory: true, multiple: false, title })
	if (Array.isArray(result)) return result[0] ?? null
	return result
}

export async function pickImage(): Promise<string | null> {
	const result = await open({
		multiple: false,
		title: 'Choose a pack icon',
		filters: [{ name: 'Image', extensions: ['png', 'jpg', 'jpeg', 'webp'] }],
	})
	if (Array.isArray(result)) return result[0] ?? null
	return result
}

export async function pickSaveMrpack(name: string): Promise<string | null> {
	return save({
		defaultPath: `${name}.mrpack`,
		filters: [{ name: 'Modrinth Modpack', extensions: ['mrpack'] }],
	})
}

export async function pickSaveZip(name: string): Promise<string | null> {
	return save({
		defaultPath: `${name}.zip`,
		filters: [{ name: 'CurseForge Modpack', extensions: ['zip'] }],
	})
}

export function revealFolder(path: string): Promise<void> {
	return openPath(path)
}

export function openExternal(url: string): Promise<void> {
	return openUrl(url)
}

export function openModrinthPage(slug: string): Promise<void> {
	return openUrl(`https://modrinth.com/mod/${slug}`)
}

export function openCurseforgePage(slug: string, projectType: string): Promise<void> {
	const cls =
		projectType === 'resourcepack'
			? 'texture-packs'
			: projectType === 'shader'
				? 'shaders'
				: 'mc-mods'
	return openUrl(`https://www.curseforge.com/minecraft/${cls}/${slug}`)
}

export const GUIDES_URL = 'https://packweave.com/guides'

export function openGuides(): Promise<void> {
	return openUrl(GUIDES_URL)
}

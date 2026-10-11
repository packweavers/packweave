import { call } from './call'
import type {
	Branches,
	Compare,
	ConflictSides,
	FileStat,
	GitChange,
	GitCommit,
	GithubAccess,
	GithubRelease,
	GitSetup,
	GitStatus,
	GraphCommit,
	MergeState,
	PackDiff,
	Remote,
	Stash,
	Tag,
	Worktree,
} from './types'

export function status(dir: string): Promise<GitStatus> {
	return call('git_status', { path: dir })
}

export function init(dir: string): Promise<void> {
	return call('git_init', { path: dir })
}

export function setup(): Promise<GitSetup> {
	return call('git_setup_state')
}

export function recheckSetup(): Promise<GitSetup> {
	return call('git_setup_recheck')
}

export function installGit(): Promise<GitSetup> {
	return call('git_setup_install')
}

export function setIdentity(name: string, email: string): Promise<GitSetup> {
	return call('git_set_identity', { name, email })
}

export function saveToken(token: string, host: string | null): Promise<string | null> {
	return call('git_token_save', { token, host })
}

export function deleteToken(host: string | null): Promise<void> {
	return call('git_token_delete', { host })
}

export function commit(
	dir: string,
	message: string,
	files: string[],
	amend: boolean,
): Promise<void> {
	return call('git_commit', { path: dir, message, files, amend })
}

export function log(dir: string, limit: number, file: string | null = null): Promise<GitCommit[]> {
	return call('git_log', { path: dir, limit, refname: null, file })
}

export function graph(
	dir: string,
	limit: number,
	skip: number,
	all: boolean,
): Promise<GraphCommit[]> {
	return call('git_graph', { path: dir, limit, skip, all })
}

export function push(dir: string, force = false, tags = false): Promise<string> {
	return call('git_push', { path: dir, force, tags })
}

export function pull(dir: string, strategy: string): Promise<string> {
	return call('git_pull', { path: dir, strategy })
}

export function fetch(dir: string): Promise<string> {
	return call('git_fetch', { path: dir })
}

export function discardFiles(dir: string, files: string[]): Promise<void> {
	return call('git_discard_files', { path: dir, files })
}

export function diffFile(dir: string, file: string): Promise<string> {
	return call('git_diff_file', { path: dir, file, staged: false })
}

export function workingStats(dir: string): Promise<FileStat[]> {
	return call('git_working_stats', { path: dir })
}

export function branches(dir: string): Promise<Branches> {
	return call('git_branches', { path: dir })
}

export function checkout(dir: string, branch: string): Promise<void> {
	return call('git_checkout', { path: dir, branch })
}

export function createBranch(
	dir: string,
	name: string,
	startPoint: string | null,
	checkoutIt: boolean,
): Promise<void> {
	return call('git_create_branch', { path: dir, name, startPoint, checkout: checkoutIt })
}

export function renameBranch(dir: string, from: string, to: string): Promise<void> {
	return call('git_rename_branch', { path: dir, old: from, new: to })
}

export function deleteBranch(dir: string, name: string, force: boolean): Promise<void> {
	return call('git_delete_branch', { path: dir, name, force })
}

export function deleteRemoteBranch(dir: string, name: string): Promise<string> {
	return call('git_delete_remote_branch', { path: dir, name })
}

export function rebase(dir: string, onto: string): Promise<string> {
	return call('git_rebase', { path: dir, name: onto })
}

export function pushBranch(
	dir: string,
	remote: string,
	branch: string,
	setUpstreamToo: boolean,
): Promise<string> {
	return call('git_push_branch', { path: dir, remote, branch, setUpstream: setUpstreamToo })
}

export function merge(dir: string, from: string, mode: string, message: string): Promise<boolean> {
	return call('git_merge', { path: dir, from, mode, message })
}

export function mergeState(dir: string): Promise<MergeState> {
	return call('git_merge_state', { path: dir })
}

export function conflictSides(dir: string, file: string): Promise<ConflictSides> {
	return call('git_conflict_sides', { path: dir, file })
}

export function resolveConflict(dir: string, file: string, side: 'ours' | 'theirs'): Promise<void> {
	return call('git_resolve_conflict', { path: dir, file, side })
}

export function resolveWith(dir: string, file: string, text: string): Promise<void> {
	return call('git_resolve_with', { path: dir, file, text })
}

export function resolveContentVersion(dir: string, file: string, version: string): Promise<void> {
	return call('git_resolve_content_version', { path: dir, file, version })
}

export function mergeAbort(dir: string): Promise<void> {
	return call('git_merge_abort', { path: dir })
}

export function mergeCommit(dir: string, message: string): Promise<void> {
	return call('git_merge_commit', { path: dir, message })
}

export function compare(dir: string, base: string, other: string): Promise<Compare> {
	return call('git_compare', { path: dir, base, other })
}

export function commitChanges(dir: string, hash: string): Promise<GitChange[]> {
	return call('git_commit_changes', { path: dir, hash })
}

export function showDiff(dir: string, hash: string, file: string): Promise<string> {
	return call('git_show_diff', { path: dir, hash, file })
}

export function showFile(dir: string, reference: string, file: string): Promise<string> {
	return call('git_show_file', { path: dir, reference, file })
}

export function packDiff(dir: string, from: string, to: string): Promise<PackDiff> {
	return call('git_pack_diff', { path: dir, from, to })
}

export function packDiffWorking(dir: string): Promise<PackDiff> {
	return call('git_pack_diff_working', { path: dir })
}

export function latestTag(dir: string): Promise<string | null> {
	return call('git_latest_tag', { path: dir })
}

export function saveChangelog(dir: string, section: string): Promise<void> {
	return call('changelog_save', { path: dir, section })
}

export function revertCommit(dir: string, hash: string): Promise<string> {
	return call('git_revert_commit', { path: dir, hash })
}

export function reset(dir: string, hash: string, mode: 'soft' | 'mixed' | 'hard'): Promise<void> {
	return call('git_reset', { path: dir, hash, mode })
}

export function cherryPick(dir: string, hash: string): Promise<string> {
	return call('git_cherry_pick', { path: dir, hash })
}

export function stashes(dir: string): Promise<Stash[]> {
	return call('git_stash_list', { path: dir })
}

export function stashPush(dir: string, message: string, includeUntracked = true): Promise<string> {
	return call('git_stash_push', { path: dir, message, includeUntracked })
}

export function stashApply(dir: string, reference: string, drop: boolean): Promise<string> {
	return call('git_stash_apply', { path: dir, reference, drop })
}

export function stashDrop(dir: string, reference: string): Promise<void> {
	return call('git_stash_drop', { path: dir, reference })
}

export function tags(dir: string): Promise<Tag[]> {
	return call('git_tags', { path: dir })
}

export function createTag(
	dir: string,
	name: string,
	message: string | null,
	target: string | null,
): Promise<void> {
	return call('git_create_tag', { path: dir, name, message, target })
}

export function deleteTag(dir: string, name: string): Promise<void> {
	return call('git_delete_tag', { path: dir, name })
}

export function pushTag(dir: string, name: string): Promise<string> {
	return call('git_push_tag', { path: dir, name })
}

export function upstreamHead(dir: string): Promise<string | null> {
	return call('git_upstream_head', { path: dir })
}

export function markTag(dir: string, name: string, commit: string): Promise<void> {
	return call('git_mark_tag', { path: dir, name, commit })
}

export function packUrl(dir: string): Promise<string | null> {
	return call('git_pack_url', { path: dir })
}

export function githubRepo(dir: string): Promise<string | null> {
	return call('git_github_repo', { path: dir })
}

export function remotes(dir: string): Promise<Remote[]> {
	return call('git_remotes', { path: dir })
}

export function addRemote(dir: string, name: string, url: string): Promise<void> {
	return call('git_add_remote', { path: dir, name, url })
}

export function setRemoteUrl(dir: string, name: string, url: string): Promise<void> {
	return call('git_set_remote_url', { path: dir, name, url })
}

export function removeRemote(dir: string, name: string): Promise<void> {
	return call('git_remove_remote', { path: dir, name })
}

export function worktrees(dir: string): Promise<Worktree[]> {
	return call('worktree_list', { path: dir })
}

export function mainFolder(dir: string): Promise<string> {
	return call('worktree_main', { path: dir })
}

export function branchFolder(dir: string, branch: string, start: string | null): Promise<string> {
	return call('branch_folder', { path: dir, branch, start })
}

export function githubAccess(dir: string, repo: string | null): Promise<GithubAccess> {
	return call('github_access', { path: dir, repo })
}

export function githubRelease(dir: string, release: GithubRelease): Promise<string> {
	return call('github_release', { path: dir, release })
}

export async function commitInfo(dir: string, hash: string): Promise<GitCommit | null> {
	const found: GitCommit[] = await call('git_log', {
		path: dir,
		limit: 1,
		refname: hash,
		file: null,
	})
	return found[0] ?? null
}

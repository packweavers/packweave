import { api } from '../../api'
import type { PullStrategy } from '../../types'
import { s, notify, type GitProvider } from './state.svelte'
import { setPref } from './prefs'
import { reloadPack } from './lifecycle'
import { KEYCHAIN } from '../features'

let pendingAuthRetry: (() => Promise<void>) | null = null

function detectProvider(url: string): GitProvider {
	if (/github\.com/i.test(url)) return 'github'
	if (/gitlab\.com/i.test(url)) return 'gitlab'
	return 'other'
}

export async function handleGitError(e: unknown, retry: () => Promise<void>, urlHint?: string) {
	const msg = String(e)
	if (!msg.startsWith('GIT_AUTH:')) {
		notify('error', msg)
		return
	}
	if (!KEYCHAIN) {
		notify(
			'error',
			'Git could not authenticate. packweave uses your computer’s Git sign-in, so set up your Git credentials (for example with the GitHub CLI or Git Credential Manager) and try again.',
		)
		return
	}
	pendingAuthRetry = retry
	let url = urlHint ?? ''
	if (!url && s.pack) {
		try {
			const rs = await api.gitRemotes(s.pack.dir)
			url = (rs.find((r) => r.name === 'origin') ?? rs[0])?.url ?? ''
		} catch {}
	}
	s.authPrompt = { provider: detectProvider(url) }
}

export async function saveAuthToken(token: string) {
	const t = token.trim()
	if (!t) return
	try {
		await api.secretSet('git_token', t)
		setPref('secret:git_token', true)
	} catch (e) {
		notify('error', `${e}`)
		return
	}
	s.authPrompt = null
	const retry = pendingAuthRetry
	pendingAuthRetry = null
	if (retry) await retry()
}

export function dismissAuth() {
	s.authPrompt = null
	pendingAuthRetry = null
}

export async function refreshGit() {
	if (!s.pack) return
	try {
		s.git = await api.gitStatus(s.pack.dir)
	} catch (e) {
		s.git = null
		notify('error', `${e}`)
	}
}

export async function gitInit() {
	if (!s.pack) return
	try {
		await api.gitInit(s.pack.dir)
		await refreshGit()
		notify('success', 'Initialized Git repository')
	} catch (e) {
		notify('error', `${e}`)
	}
}

export async function gitCommit(message: string, files: string[], amend = false) {
	if (!s.pack || !message.trim() || (!files.length && !amend)) return
	s.busy = true
	try {
		await api.gitCommit(s.pack.dir, message.trim(), files, amend)
		await refreshGit()
		notify(
			'success',
			amend
				? 'Amended last commit'
				: `Committed ${files.length} ${files.length === 1 ? 'file' : 'files'}`,
		)
	} catch (e) {
		notify('error', `${e}`)
	} finally {
		s.busy = false
	}
}

export async function gitCommitPush(message: string, files: string[], amend = false) {
	if (!s.pack || !message.trim() || (!files.length && !amend)) return
	s.busy = true
	try {
		await api.gitCommit(s.pack.dir, message.trim(), files, amend)
		const out = await api.gitPush(s.pack.dir, amend)
		await refreshGit()
		notify('success', out.trim() || 'Committed and pushed')
	} catch (e) {
		await handleGitError(e, () => gitPush(amend))
	} finally {
		s.busy = false
	}
}

export async function gitPush(force = false, tags = false) {
	if (!s.pack) return
	s.busy = true
	try {
		const out = await api.gitPush(s.pack.dir, force, tags)
		await refreshGit()
		notify('success', out.trim() || 'Pushed')
	} catch (e) {
		await handleGitError(e, () => gitPush(force, tags))
	} finally {
		s.busy = false
	}
}

export async function gitPull(strategy: PullStrategy = 'ff') {
	if (!s.pack) return
	s.busy = true
	try {
		const out = await api.gitPull(s.pack.dir, strategy)
		await reloadPack()
		notify('success', out.trim() || 'Pulled')
	} catch (e) {
		await handleGitError(e, () => gitPull(strategy))
	} finally {
		s.busy = false
	}
}

export async function gitDiscard() {
	if (!s.pack) return
	s.busy = true
	try {
		await api.gitDiscard(s.pack.dir)
		await reloadPack()
		notify('success', 'Discarded all changes')
	} catch (e) {
		notify('error', `${e}`)
	} finally {
		s.busy = false
	}
}

export async function gitRevert(files: string[]) {
	if (!s.pack || !files.length) return
	s.busy = true
	try {
		await api.gitRevert(s.pack.dir, files)
		await reloadPack()
		notify('success', `Reverted ${files.length} ${files.length === 1 ? 'file' : 'files'}`)
	} catch (e) {
		notify('error', `${e}`)
	} finally {
		s.busy = false
	}
}

export async function gitFetch() {
	if (!s.pack) return
	s.busy = true
	try {
		await api.gitFetch(s.pack.dir)
		await refreshGit()
		notify('success', 'Fetched from remote')
	} catch (e) {
		await handleGitError(e, () => gitFetch())
	} finally {
		s.busy = false
	}
}

export async function gitCheckout(branch: string) {
	if (!s.pack) return
	s.busy = true
	try {
		await api.gitCheckout(s.pack.dir, branch)
		await reloadPack()
		notify('success', `Switched to ${branch}`)
	} catch (e) {
		notify('error', `${e}`)
	} finally {
		s.busy = false
	}
}

export async function gitCreateBranch(name: string, startPoint?: string, checkout = true) {
	if (!s.pack || !name.trim()) return
	s.busy = true
	try {
		await api.gitCreateBranch(s.pack.dir, name.trim(), startPoint, checkout)
		if (checkout) await reloadPack()
		else await refreshGit()
		notify('success', `Created branch ${name.trim()}`)
	} catch (e) {
		notify('error', `${e}`)
	} finally {
		s.busy = false
	}
}

export async function gitRenameBranch(oldName: string, newName: string) {
	if (!s.pack || !newName.trim()) return
	s.busy = true
	try {
		await api.gitRenameBranch(s.pack.dir, oldName, newName.trim())
		await refreshGit()
		notify('success', `Renamed to ${newName.trim()}`)
	} catch (e) {
		notify('error', `${e}`)
	} finally {
		s.busy = false
	}
}

export async function gitDeleteBranch(name: string, force = false) {
	if (!s.pack) return
	s.busy = true
	try {
		await api.gitDeleteBranch(s.pack.dir, name, force)
		await refreshGit()
		notify('success', `Deleted branch ${name}`)
	} catch (e) {
		notify('error', `${e}`)
	} finally {
		s.busy = false
	}
}

export async function gitDeleteRemoteBranch(name: string) {
	if (!s.pack) return
	s.busy = true
	try {
		await api.gitDeleteRemoteBranch(s.pack.dir, name)
		await refreshGit()
		notify('success', `Deleted ${name}`)
	} catch (e) {
		await handleGitError(e, () => gitDeleteRemoteBranch(name))
	} finally {
		s.busy = false
	}
}

export async function gitMerge(name: string) {
	if (!s.pack) return
	s.busy = true
	try {
		const out = await api.gitMerge(s.pack.dir, name)
		await reloadPack()
		notify('success', out.trim() || `Merged ${name}`)
	} catch (e) {
		notify('error', `${e}`)
	} finally {
		s.busy = false
	}
}

export async function gitRebase(name: string) {
	if (!s.pack) return
	s.busy = true
	try {
		const out = await api.gitRebase(s.pack.dir, name)
		await reloadPack()
		notify('success', out.trim() || `Rebased onto ${name}`)
	} catch (e) {
		notify('error', `${e}`)
	} finally {
		s.busy = false
	}
}

export async function gitSetUpstream(upstream: string) {
	if (!s.pack) return
	s.busy = true
	try {
		await api.gitSetUpstream(s.pack.dir, upstream)
		await refreshGit()
		notify('success', `Tracking ${upstream}`)
	} catch (e) {
		notify('error', `${e}`)
	} finally {
		s.busy = false
	}
}

export async function gitPushBranch(remote: string, branch: string) {
	if (!s.pack) return
	s.busy = true
	try {
		const out = await api.gitPushBranch(s.pack.dir, remote, branch)
		await refreshGit()
		notify('success', out.trim() || `Pushed ${branch}`)
	} catch (e) {
		await handleGitError(e, () => gitPushBranch(remote, branch))
	} finally {
		s.busy = false
	}
}

export async function gitRevertCommit(hash: string) {
	if (!s.pack) return
	s.busy = true
	try {
		await api.gitRevertCommit(s.pack.dir, hash)
		await reloadPack()
		notify('success', 'Reverted commit')
	} catch (e) {
		notify('error', `${e}`)
	} finally {
		s.busy = false
	}
}

export async function gitReset(hash: string, mode: 'soft' | 'mixed' | 'hard') {
	if (!s.pack) return
	s.busy = true
	try {
		await api.gitReset(s.pack.dir, hash, mode)
		await reloadPack()
		notify('success', `Reset (${mode}) to ${hash.slice(0, 7)}`)
	} catch (e) {
		notify('error', `${e}`)
	} finally {
		s.busy = false
	}
}

export async function gitCherryPick(hash: string) {
	if (!s.pack) return
	s.busy = true
	try {
		await api.gitCherryPick(s.pack.dir, hash)
		await reloadPack()
		notify('success', `Cherry-picked ${hash.slice(0, 7)}`)
	} catch (e) {
		notify('error', `${e}`)
	} finally {
		s.busy = false
	}
}

export async function gitStash(message: string, includeUntracked: boolean) {
	if (!s.pack) return
	s.busy = true
	try {
		await api.gitStashPush(s.pack.dir, message, includeUntracked)
		await reloadPack()
		notify('success', 'Stashed changes')
	} catch (e) {
		notify('error', `${e}`)
	} finally {
		s.busy = false
	}
}

export async function gitStashApply(reference: string, drop: boolean) {
	if (!s.pack) return
	s.busy = true
	try {
		await api.gitStashApply(s.pack.dir, reference, drop)
		await reloadPack()
		notify('success', drop ? 'Popped stash' : 'Applied stash')
	} catch (e) {
		notify('error', `${e}`)
	} finally {
		s.busy = false
	}
}

export async function gitStashDrop(reference: string) {
	if (!s.pack) return
	s.busy = true
	try {
		await api.gitStashDrop(s.pack.dir, reference)
		notify('success', 'Dropped stash')
	} catch (e) {
		notify('error', `${e}`)
	} finally {
		s.busy = false
	}
}

export async function gitCreateTag(name: string, message?: string, target?: string) {
	if (!s.pack || !name.trim()) return
	s.busy = true
	try {
		await api.gitCreateTag(s.pack.dir, name.trim(), message, target)
		notify('success', `Tagged ${name.trim()}`)
	} catch (e) {
		notify('error', `${e}`)
	} finally {
		s.busy = false
	}
}

export async function gitDeleteTag(name: string) {
	if (!s.pack) return
	s.busy = true
	try {
		await api.gitDeleteTag(s.pack.dir, name)
		notify('success', `Deleted tag ${name}`)
	} catch (e) {
		notify('error', `${e}`)
	} finally {
		s.busy = false
	}
}

export async function gitPushTag(name: string) {
	if (!s.pack) return
	s.busy = true
	try {
		const out = await api.gitPushTag(s.pack.dir, name)
		notify('success', out.trim() || `Pushed tag ${name}`)
	} catch (e) {
		await handleGitError(e, () => gitPushTag(name))
	} finally {
		s.busy = false
	}
}

export async function gitDiscardFile(file: string) {
	if (!s.pack) return
	s.busy = true
	try {
		await api.gitDiscardFile(s.pack.dir, file)
		await reloadPack()
	} catch (e) {
		notify('error', `${e}`)
	} finally {
		s.busy = false
	}
}

export async function gitResolveConflict(file: string, side: 'ours' | 'theirs') {
	if (!s.pack) return
	s.busy = true
	try {
		await api.gitResolveConflict(s.pack.dir, file, side)
		await refreshGit()
		notify('success', `Resolved ${file} (${side})`)
	} catch (e) {
		notify('error', `${e}`)
	} finally {
		s.busy = false
	}
}

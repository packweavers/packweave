import CommitPushDialog from '@/dialogs/CommitPushDialog.svelte'
import ConfirmDialog from '@/dialogs/ConfirmDialog.svelte'
import { app } from '@/lib/state/app.svelte'
import { content } from '@/lib/state/content.svelte'
import { dialogs } from '@/lib/state/dialogs.svelte'
import { git } from '@/lib/state/git.svelte'
import type { ReleasePrefs } from '@/lib/state/prefs'
import { tasks } from '@/lib/state/tasks.svelte'
import { toasts } from '@/lib/state/toasts.svelte'
import { ws } from '@/lib/state/workspace.svelte'
import { packDistToken } from '@/lib/tauri/app'
import * as gitApi from '@/lib/tauri/git'
import * as releaseApi from '@/lib/tauri/release'
import type { Channel } from '@/lib/tauri/types'
import { openExternal, openFolder, pickFolder } from '@/lib/tauri/windows'
import { plural } from '@/lib/util/format'
import { join } from '@/lib/util/path'
import { githubRepo, slugify } from '@/lib/util/text'

import { withGit } from './git'

export interface ReleasePlan {
	version: string
	channel: Channel
	notes: string
}

interface ExportJob {
	label: string
	file: string
	run: (dir: string, out: string) => Promise<unknown>
}

export function onGithub(release: ReleasePrefs): boolean {
	return release.publish.github || release.updates === 'github'
}

function onModrinth(release: ReleasePrefs): boolean {
	return release.updates === 'none' && release.publish.modrinth
}

function onCurseforge(release: ReleasePrefs): boolean {
	return release.updates === 'none' && release.publish.curseforge
}

export function publishes(release: ReleasePrefs): boolean {
	return onGithub(release) || onModrinth(release) || onCurseforge(release)
}

async function releaseRepo(dir: string, release: ReleasePrefs): Promise<string | null> {
	return githubRepo(release.repo) ?? (await gitApi.githubRepo(dir))
}

async function publishRepo(dir: string, release: ReleasePrefs): Promise<string | null> {
	return githubRepo(release.publishRepo) ?? (await releaseRepo(dir, release))
}

async function updateUrl(dir: string, release: ReleasePrefs): Promise<string | null> {
	if (release.updates === 'url') return release.url.trim() || (await gitApi.packUrl(dir))
	if (release.updates !== 'github') return null
	const repo = await releaseRepo(dir, release)
	return repo ? `release+https://github.com/${repo}` : null
}

async function plannedExports(
	dir: string,
	release: ReleasePrefs,
	version: string,
): Promise<ExportJob[]> {
	const { exports } = release
	const url = await updateUrl(dir, release)
	const base = `${slugify(ws.name)}-${version}`
	const jobs: ExportJob[] = []
	if (exports.mrpack) {
		jobs.push({
			label: 'Modrinth pack',
			file: `${base}.mrpack`,
			run: (dir, out) =>
				url
					? releaseApi.exportMrpackSelfUpdate(dir, out, url)
					: releaseApi.exportMrpack(dir, out, 'common'),
		})
	}
	if (exports.curseforge) {
		jobs.push({
			label: 'CurseForge pack',
			file: `${base}-curseforge.zip`,
			run: (dir, out) =>
				url
					? releaseApi.exportCurseforgeSelfUpdate(dir, out, url)
					: releaseApi.exportCurseforge(dir, out, 'common'),
		})
	}
	if (exports.instance) {
		jobs.push({
			label: 'Prism instance',
			file: `${base}-prism.zip`,
			run: (dir, out) =>
				url ? releaseApi.exportDist(dir, out, url) : releaseApi.exportInstance(dir, out, 'client'),
		})
	}
	if (exports.server) {
		jobs.push({
			label: 'Server pack',
			file: `${base}-server.mrpack`,
			run: (dir, out) => releaseApi.exportMrpack(dir, out, 'server'),
		})
	}
	if (exports.client) {
		jobs.push({
			label: 'Client pack',
			file: `${base}-client.mrpack`,
			run: (dir, out) => releaseApi.exportMrpack(dir, out, 'client'),
		})
	}
	return jobs
}

async function runExports(dir: string, jobs: ExportJob[], folder: string): Promise<string[]> {
	const written: string[] = []
	for (const job of jobs) {
		const out = join(folder, job.file)
		await tasks.run(`Exporting ${job.label}`, () => job.run(dir, out))
		written.push(out)
	}
	return written
}

async function updatesReady(dir: string, release: ReleasePrefs): Promise<boolean> {
	if (release.updates === 'none' || (await updateUrl(dir, release))) return true
	toasts.info(
		release.updates === 'url' ? 'Enter the update URL first' : 'Enter the GitHub repository first',
	)
	return false
}

export async function exportAll(plan: ReleasePlan): Promise<void> {
	const dir = ws.dir
	const main = ws.mainDir ?? dir
	if (!dir || !main) return
	const release = app.pack(main).release
	if (!(await updatesReady(dir, release))) return
	const jobs = await plannedExports(dir, release, plan.version)
	if (jobs.length === 0) {
		toasts.info('Pick at least one export first')
		return
	}
	const folder = await pickFolder('Choose where to save the exports')
	if (!folder) return
	try {
		const written = await runExports(dir, jobs, folder)
		toasts.show(`Exported ${plural(written.length, 'file', 'files')}`, [
			{ label: 'Show', run: () => void openFolder(folder) },
		])
	} catch (error) {
		toasts.error(error)
	}
}

async function writeVersion(plan: ReleasePlan): Promise<void> {
	const manifest = ws.manifest
	if (!manifest || (manifest.version === plan.version && manifest.channel === plan.channel)) return
	await ws.saveManifest({ ...manifest, version: plan.version, channel: plan.channel })
}

async function commitAndPush(answer: { message: string; files: string[] }): Promise<void> {
	if (answer.files.length > 0) await git.commit(answer.message, answer.files, false)
	if ((git.status?.ahead ?? 0) > 0 || !git.status?.upstream) await git.push()
}

function githubStep(repo: string, release: ReleasePrefs, exports: number): string {
	const attached = [
		release.updates !== 'none' ? 'the update files' : null,
		exports > 0 ? plural(exports, 'export', 'exports') : null,
	].filter((part) => part !== null)
	const step = `Create the GitHub release on ${repo}`
	return attached.length > 0 ? `${step} with ${attached.join(' and ')}` : step
}

async function steps(
	dir: string,
	release: ReleasePrefs,
	version: string,
	repo: string | null,
	at: string | null,
): Promise<string[]> {
	const jobs = await plannedExports(dir, release, version)
	const list: string[] = []
	if (repo && at) list.push(`Tag v${version} on ${at}`)
	if (jobs.length > 0) list.push(`Export ${plural(jobs.length, 'file', 'files')}`)
	if (repo) list.push(githubStep(repo, release, jobs.length))
	if (onModrinth(release)) list.push('Upload to Modrinth')
	if (onCurseforge(release)) list.push('Upload to CurseForge')
	return list
}

export async function publishSteps(version: string): Promise<string[]> {
	const dir = ws.dir
	const main = ws.mainDir ?? dir
	if (!dir || !main) return []
	const release = app.pack(main).release
	if (!publishes(release)) return []
	const repo = onGithub(release) ? await publishRepo(dir, release) : null
	return steps(dir, release, version, repo, git.isRepo ? 'the pushed commit' : null)
}

async function tokenWarning(dir: string, repo: string): Promise<string | undefined> {
	const [access, token] = await Promise.all([
		gitApi.githubAccess(dir, repo).catch(tasks.or('Checking the GitHub repository', null)),
		packDistToken(dir).catch(tasks.or('Reading the update token', null)),
	])
	if (access?.visibility !== 'private' || token) return undefined
	return `${repo} is private and there's no read-only token, so players can't download updates.`
}

function blocker(release: ReleasePrefs, repo: string | null): string | null {
	if (onGithub(release) && !repo) return 'Enter the GitHub repository first'
	if (onModrinth(release) && !release.modrinthProject) return 'Enter the Modrinth project ID first'
	if (onCurseforge(release) && !release.curseforgeProject) {
		return 'Enter the CurseForge project ID first'
	}
	if (git.isRepo && !git.remoteName) return 'Add a remote first'
	return null
}

async function pushedCommit(dir: string, plan: ReleasePlan, tag: string): Promise<string | null> {
	if (!git.isRepo) return null
	await writeVersion(plan)
	await Promise.all([git.refresh(), content.refreshChanges()])
	if (git.changes.length > 0 || (git.status?.ahead ?? 0) > 0 || !git.status?.upstream) {
		const answer = await dialogs.open(CommitPushDialog, {
			title: 'Commit and push before publishing?',
			message: `Release ${tag}`,
			selected: git.changes.some((c) => c.path === 'modpack.json') ? ['modpack.json'] : [],
		})
		if (!answer) return ''
		if (answer !== 'skip') {
			const pushed = await withGit(async () => {
				await commitAndPush(answer)
				return true
			})
			if (!pushed) return ''
		}
	}
	const commit = await gitApi.upstreamHead(dir)
	if (!commit) {
		toasts.info(`Push ${git.branch ?? 'this branch'} first`)
		return ''
	}
	const pushed = JSON.parse(await gitApi.showFile(dir, commit, 'modpack.json')) as {
		version?: string
	}
	if (pushed.version !== plan.version) {
		toasts.info(
			`The pushed modpack.json is version ${pushed.version ?? 'unknown'}. Push the version change first`,
		)
		return ''
	}
	return commit
}

export async function publish(plan: ReleasePlan): Promise<void> {
	const dir = ws.dir
	const main = ws.mainDir ?? dir
	if (!dir || !main || !ws.manifest) return
	const release = app.pack(main).release
	if (!publishes(release)) return
	if (!(await updatesReady(dir, release))) return
	const repo = onGithub(release) ? await publishRepo(dir, release) : null
	const blocked = blocker(release, repo)
	if (blocked) {
		toasts.info(blocked)
		return
	}
	const tag = `v${plan.version}`
	try {
		const commit = await pushedCommit(dir, plan, tag)
		if (commit === '') return
		const short = commit ? commit.slice(0, 7) : null
		const updateRepo = release.updates === 'github' ? await releaseRepo(dir, release) : null
		const warning = updateRepo ? await tokenWarning(dir, updateRepo) : undefined
		const ok = await dialogs.open(ConfirmDialog, {
			title: `Publish ${tag}?`,
			message: [
				commit ? `Publishes ${git.status?.upstream ?? 'the pushed branch'} at ${short}.` : null,
				warning,
			]
				.filter((part) => !!part)
				.join(' '),
			steps: await steps(dir, release, plan.version, repo, short),
			confirm: 'Publish',
		})
		if (!ok) return
		if (!commit) await writeVersion(plan)
		const remoteRepo = commit ? await gitApi.githubRepo(dir).catch(() => null) : null
		if (commit && repo) await gitApi.markTag(dir, tag, commit)
		const built = commit
			? await tasks.run('Checking out the release', () => releaseApi.releaseCheckout(dir, commit))
			: dir
		let page: string | null = null
		let folder: string | null = null
		let written: string[] = []
		try {
			const jobs = await plannedExports(dir, release, plan.version)
			if (jobs.length > 0) {
				folder = repo
					? await releaseApi.releaseFolder()
					: await pickFolder('Choose where to save the exports')
			}
			written = folder ? await runExports(built, jobs, folder) : []
			if (repo) {
				const url = await updateUrl(dir, release)
				const sameRepo = !!remoteRepo && remoteRepo.toLowerCase() === repo.toLowerCase()
				page = await tasks.run('Creating the GitHub release', () =>
					gitApi.githubRelease(built, {
						repo,
						tag,
						name: `${ws.name} ${tag}`,
						body: plan.notes,
						updates: release.updates !== 'none',
						url,
						prerelease: plan.channel !== 'release' && release.updates !== 'github',
						assets: written,
						target: commit && sameRepo ? commit : null,
					}),
				)
			}
			if (onModrinth(release)) {
				await tasks.run('Uploading to Modrinth', () =>
					releaseApi.publish(
						'modrinth',
						built,
						release.modrinthProject,
						plan.version,
						plan.notes,
						'common',
					),
				)
			}
			if (onCurseforge(release)) {
				await tasks.run('Uploading to CurseForge', () =>
					releaseApi.publish(
						'curseforge',
						built,
						release.curseforgeProject,
						plan.version,
						plan.notes,
						'common',
					),
				)
			}
		} finally {
			if (built !== dir) {
				await releaseApi
					.releaseCheckoutRemove(dir, built)
					.catch(tasks.or('Removing the release checkout', undefined))
			}
		}
		await git.refreshAll()
		const saved = !repo && written.length > 0 ? folder : null
		if (page) {
			toasts.show(`Published ${tag}`, [{ label: 'Show', run: () => void openExternal(page!) }])
		} else if (saved) {
			toasts.show(`Published ${tag}`, [
				{ label: 'Show exports', run: () => void openFolder(saved) },
			])
		} else {
			toasts.show(`Published ${tag}`)
		}
	} catch (error) {
		toasts.error(error)
	}
}

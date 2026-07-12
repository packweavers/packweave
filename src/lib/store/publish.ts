import { api, pickFolder, pickSaveMrpack, pickSaveZip, revealFolder } from '../../api'
import type { PackEnv } from '../../types'
import { s, notify } from './state.svelte'

export async function exportPack(env: PackEnv = 'common') {
	if (!s.pack || !s.hasLock) return
	const dir = s.pack.dir
	const suffix = env === 'common' ? '' : `-${env}`
	const out = await pickSaveMrpack(`${s.pack.manifest.name}${suffix}`)
	if (!out) return
	s.busy = true
	try {
		await api.exportMrpack(dir, out, env)
		notify('success', `Exported ${out.split(/[/\\]/).pop()}`)
	} catch (e) {
		notify('error', `${e}`)
	} finally {
		s.busy = false
	}
}

export async function exportCurseforge(env: PackEnv = 'common') {
	if (!s.pack || !s.hasLock) return
	const dir = s.pack.dir
	const suffix = env === 'common' ? '' : `-${env}`
	const out = await pickSaveZip(`${s.pack.manifest.name}${suffix}`)
	if (!out) return
	s.busy = true
	try {
		await api.exportCurseforge(dir, out, env)
		notify('success', `Exported ${out.split(/[/\\]/).pop()}`)
	} catch (e) {
		notify('error', `${e}`)
	} finally {
		s.busy = false
	}
}

export async function exportInstance(env: PackEnv = 'common') {
	if (!s.pack || !s.hasLock) return
	const dir = s.pack.dir
	const suffix = env === 'common' ? '' : `-${env}`
	const out = await pickSaveZip(`${s.pack.manifest.name}${suffix}`)
	if (!out) return
	s.busy = true
	try {
		await api.exportInstance(dir, out, env)
		notify('success', `Exported ${out.split(/[/\\]/).pop()}`)
	} catch (e) {
		notify('error', `${e}`)
	} finally {
		s.busy = false
	}
}

export async function exportMrpackSelfUpdate(url: string) {
	if (!s.pack || !url.trim()) return
	const out = await pickSaveMrpack(`${s.pack.manifest.name}`)
	if (!out) return
	s.busy = true
	try {
		await api.exportMrpackSelfUpdate(s.pack.dir, out, url.trim())
		notify('success', `Exported ${out.split(/[/\\]/).pop()}`)
	} catch (e) {
		notify('error', `${e}`)
	} finally {
		s.busy = false
	}
}

export async function exportCurseforgeSelfUpdate(url: string) {
	if (!s.pack || !url.trim()) return
	const out = await pickSaveZip(`${s.pack.manifest.name}`)
	if (!out) return
	s.busy = true
	try {
		await api.exportCurseforgeSelfUpdate(s.pack.dir, out, url.trim())
		notify('success', `Exported ${out.split(/[/\\]/).pop()}`)
	} catch (e) {
		notify('error', `${e}`)
	} finally {
		s.busy = false
	}
}

export async function checkForUpdate() {
	try {
		s.updateInfo = await api.checkUpdate()
	} catch {
		s.updateInfo = null
	}
}

export function dismissUpdate() {
	s.updateInfo = null
}

export async function installUpdate() {
	s.busy = true
	try {
		notify('info', 'Downloading update… the app will restart.')
		await api.installUpdate()
	} catch (e) {
		notify('error', `${e}`)
		s.busy = false
	}
}

export async function publishToPlatform(
	target: string,
	projectId: string,
	versionNumber: string,
	changelog: string,
	env: PackEnv,
): Promise<boolean> {
	if (!s.pack || !s.hasLock) return false
	s.busy = true
	try {
		const out = await api.publishPack(
			target,
			s.pack.dir,
			projectId.trim(),
			versionNumber.trim(),
			changelog,
			env,
		)
		notify('success', out || 'Published')
		return true
	} catch (e) {
		notify('error', `${e}`)
		return false
	} finally {
		s.busy = false
	}
}

export async function createGithubRelease(
	tag: string,
	name: string,
	body: string,
): Promise<boolean> {
	if (!s.pack) return false
	try {
		await api.githubRelease(s.pack.dir, tag, name, body)
		notify('success', `Created GitHub release ${tag}`)
		return true
	} catch (e) {
		notify('warning', `GitHub release skipped: ${e}`)
		return false
	}
}

export async function publishPack(packUrl: string) {
	if (!s.pack || !s.hasLock) return
	const url = packUrl.trim()
	if (!url) return
	const out = await pickFolder('Choose where to save the importable pack')
	if (!out) return
	const dir = s.pack.dir
	s.busy = true
	try {
		await api.exportDist(dir, out, url)
		notify(
			'success',
			`Saved the importable pack. Share the .zip. Players sync from your committed repo each launch.`,
		)
		await revealFolder(out)
	} catch (e) {
		notify('error', `${e}`)
	} finally {
		s.busy = false
	}
}

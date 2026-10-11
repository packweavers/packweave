import { call } from './call'

type ExportSide = 'common' | 'client' | 'server'

export function exportMrpack(dir: string, output: string, env: ExportSide): Promise<void> {
	return call('export_mrpack', { path: dir, output, env })
}

export function exportCurseforge(dir: string, output: string, env: ExportSide): Promise<void> {
	return call('export_curseforge', { path: dir, output, env })
}

export function exportInstance(dir: string, output: string, env: ExportSide): Promise<void> {
	return call('export_instance', { path: dir, output, env })
}

export function includePackweaver(dir: string): Promise<void> {
	return call('include_packweaver', { path: dir })
}

export function releaseFolder(): Promise<string> {
	return call('release_folder')
}

export function releaseCheckout(dir: string, commit: string): Promise<string> {
	return call('release_checkout', { path: dir, commit })
}

export function releaseCheckoutRemove(dir: string, folder: string): Promise<void> {
	return call('release_checkout_remove', { path: dir, folder })
}

export function exportMrpackSelfUpdate(dir: string, output: string, url: string): Promise<void> {
	return call('export_mrpack_selfupdate', { path: dir, output, url })
}

export function exportCurseforgeSelfUpdate(
	dir: string,
	output: string,
	url: string,
): Promise<void> {
	return call('export_curseforge_selfupdate', { path: dir, output, url })
}

export function exportDist(dir: string, output: string, packUrl: string): Promise<void> {
	return call('export_dist', { path: dir, output, packUrl })
}

export function publish(
	target: 'modrinth' | 'curseforge',
	dir: string,
	projectId: string,
	versionNumber: string,
	changelog: string,
	env: ExportSide,
): Promise<string> {
	return call('publish_pack', { target, path: dir, projectId, versionNumber, changelog, env })
}

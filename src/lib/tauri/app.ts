import { call } from './call'
import type { AppInfo, UpdateInfo } from './types'

export function readPrefs(): Promise<string> {
	return call('read_prefs')
}

export function writePrefs(content: string): Promise<void> {
	return call('write_prefs', { content })
}

export function setSecret(key: string, value: string): Promise<void> {
	return call('secret_set', { key, value })
}

export function deleteSecret(key: string): Promise<void> {
	return call('secret_delete', { key })
}

export function secretStatus(keys: string[]): Promise<boolean[]> {
	return call('secret_status', { keys })
}

export function appInfo(): Promise<AppInfo> {
	return call('app_info')
}

export function checkUpdate(): Promise<UpdateInfo | null> {
	return call('check_update')
}

export function installUpdate(): Promise<void> {
	return call('install_update')
}

export function packDistToken(dir: string): Promise<string | null> {
	return call('pack_dist_token', { path: dir })
}

export function savePackDistToken(dir: string, token: string): Promise<void> {
	return call('save_pack_dist_token', { path: dir, token })
}

export function checkPackDistToken(dir: string, repo: string): Promise<boolean | null> {
	return call('check_pack_dist_token', { path: dir, repo })
}

export function cacheSize(): Promise<number> {
	return call('cache_size')
}

export function clearCache(): Promise<number> {
	return call('clear_cache')
}

import { call } from './call'
import type {
	DetectedInstance,
	IgnoreKind,
	InstanceJar,
	LogChunk,
	PackResolved,
	Running,
	SyncIgnored,
	SyncOp,
	SyncReport,
} from './types'

export function bindInstance(dir: string, gameDir: string): Promise<DetectedInstance> {
	return call('bind_instance', { path: dir, instance: gameDir })
}

export function unbindInstance(dir: string): Promise<void> {
	return call('unbind_instance', { path: dir })
}

export function binding(dir: string): Promise<DetectedInstance | null> {
	return call('get_binding', { path: dir })
}

export function syncStatus(dir: string): Promise<SyncReport> {
	return call('sync_status', { path: dir })
}

export function applySync(dir: string, ops: SyncOp[], label: string): Promise<PackResolved> {
	return call('apply_sync', { path: dir, ops, label })
}

export function defaultIgnore(kind: IgnoreKind): Promise<string> {
	return call('default_ignore', { kind })
}

export function instanceJars(dir: string): Promise<InstanceJar[]> {
	return call('instance_jars', { path: dir })
}

export function setInstanceJars(dir: string, enable: string[], disable: string[]): Promise<void> {
	return call('set_instance_jars', { path: dir, enable, disable })
}

export function ignoreInSync(dir: string, items: SyncIgnored[]): Promise<SyncIgnored[]> {
	return call('sync_ignore', { path: dir, items })
}

export function unignoreInSync(dir: string, keys: string[]): Promise<SyncIgnored[]> {
	return call('sync_unignore', { path: dir, keys })
}

export function syncFileDiff(dir: string, rel: string, kind: string): Promise<string> {
	return call('sync_file_diff', { path: dir, rel, kind })
}

export function autoPushFile(dir: string, rel: string, original: string | null): Promise<boolean> {
	return call('auto_push_file', { path: dir, rel, original })
}

export function readIgnore(dir: string, kind: IgnoreKind): Promise<string> {
	return call('read_ignore', { path: dir, kind })
}

export function writeIgnore(dir: string, kind: IgnoreKind, content: string): Promise<void> {
	return call('write_ignore', { path: dir, kind, content })
}

export function addIgnorePatterns(
	dir: string,
	kind: IgnoreKind,
	patterns: string[],
): Promise<void> {
	return call('add_ignore_patterns', { path: dir, kind, patterns })
}

export function detectInstances(): Promise<DetectedInstance[]> {
	return call('detect_instances')
}

export function resolveInstanceFolder(folder: string): Promise<DetectedInstance> {
	return call('resolve_instance_folder', { path: folder })
}

export function running(dir: string): Promise<Running | null> {
	return call('instance_running', { path: dir })
}

export function readGameLog(dir: string, offset: number): Promise<LogChunk> {
	return call('read_game_log', { path: dir, offset })
}

export function createPrismInstance(dir: string): Promise<DetectedInstance> {
	return call('create_prism_instance', { path: dir })
}

export function canCopyInstance(gameDir: string): Promise<boolean> {
	return call('can_copy_instance', { gameDir })
}

export function copyInstance(gameDir: string, name: string): Promise<DetectedInstance> {
	return call('copy_instance', { gameDir, name })
}

export function deleteInstance(gameDir: string): Promise<void> {
	return call('delete_instance', { gameDir })
}

import { call } from './call'
import type {
	AddItem,
	AvailableUpdate,
	BulkLookup,
	ChangePreview,
	DroppedFile,
	DroppedScan,
	ImpactReport,
	LocalContent,
	PackResolved,
	ProviderId,
	Rechecked,
	UndoResult,
	UndoState,
	VersionPin,
} from './types'

export function addContent(dir: string, items: AddItem[]): Promise<PackResolved> {
	return call('add_content', { path: dir, items })
}

export function bulkLookup(dir: string, provider: ProviderId, text: string): Promise<BulkLookup> {
	return call('bulk_lookup', { path: dir, provider, text })
}

export function identifyDropped(dir: string, files: string[]): Promise<DroppedScan> {
	return call('identify_dropped', { path: dir, files })
}

export function addDropped(dir: string, items: DroppedFile[]): Promise<PackResolved> {
	return call('add_dropped', { path: dir, items })
}

export function recheckDroppedFiles(dir: string): Promise<Rechecked> {
	return call('recheck_dropped', { path: dir })
}

export function removeContent(dir: string, keys: string[]): Promise<PackResolved> {
	return call('remove_content', { path: dir, keys })
}

export function setDisabled(dir: string, keys: string[], disabled: boolean): Promise<PackResolved> {
	return call('set_content_disabled', { path: dir, keys, disabled })
}

export function addAltSources(
	dir: string,
	sources: { key: string; provider: ProviderId; id: string }[],
): Promise<PackResolved> {
	return call('add_alt_sources', { path: dir, sources })
}

export function setSourceVersion(
	dir: string,
	key: string,
	provider: ProviderId,
	version: string | null,
): Promise<PackResolved> {
	return call('set_source_version', { path: dir, key, provider, version })
}

export function setBundled(dir: string, keys: string[], bundled: boolean): Promise<PackResolved> {
	return call('set_content_bundled', { path: dir, keys, bundled })
}

export function setPinned(dir: string, keys: string[], pinned: boolean): Promise<PackResolved> {
	return call('set_content_pinned', { path: dir, keys, pinned })
}

export function setExplicit(dir: string, key: string, explicit: boolean): Promise<PackResolved> {
	return call('set_content_explicit', { path: dir, key, explicit })
}

export function setVersions(dir: string, updates: VersionPin[]): Promise<PackResolved> {
	return call('set_content_versions', { path: dir, updates })
}

export function setPreferredSource(
	dir: string,
	keys: string[],
	provider: ProviderId,
): Promise<PackResolved> {
	return call('set_preferred_source', { path: dir, keys, provider })
}

export function removeAltSource(
	dir: string,
	key: string,
	provider: ProviderId,
): Promise<PackResolved> {
	return call('remove_alt_source', { path: dir, key, provider })
}

export function resolvePack(dir: string, force = false): Promise<PackResolved> {
	return call('resolve_pack', { path: dir, force })
}

export function previewChange(dir: string, change: ChangePreview): Promise<ImpactReport> {
	return call('preview_change', { path: dir, change })
}

export function checkUpdates(dir: string): Promise<AvailableUpdate[]> {
	return call('check_updates', { path: dir })
}

export function listUnpublished(dir: string): Promise<LocalContent[]> {
	return call('list_unpublished', { path: dir })
}

export function undo(dir: string): Promise<UndoResult | null> {
	return call('undo', { path: dir })
}

export function redo(dir: string): Promise<UndoResult | null> {
	return call('redo', { path: dir })
}

export function undoState(dir: string): Promise<UndoState> {
	return call('undo_state', { path: dir })
}

import { call } from './call'
import type {
	CatalogHit,
	ConvertCandidate,
	LoaderVersions,
	ModMeta,
	ProjectDetails,
	ProjectType,
	ProviderId,
	ProviderInfo,
	VersionInfo,
	VersionNotes,
} from './types'

export function listProviders(): Promise<ProviderInfo[]> {
	return call('list_providers')
}

export interface SearchQuery {
	provider: ProviderId
	query: string
	minecraft: string
	loader: string
	projectType: ProjectType
	offset: number
	limit: number
}

export function search(q: SearchQuery): Promise<CatalogHit[]> {
	return call('search', { ...q })
}

export function projectDetails(provider: ProviderId, projectId: string): Promise<ProjectDetails> {
	return call('project_details', { provider, projectId })
}

export function versionChangelog(
	provider: ProviderId,
	projectId: string,
	versionId: string,
): Promise<string> {
	return call('version_changelog', { provider, projectId, versionId })
}

export function updateChangelogs(
	dir: string,
	projectId: string,
	toVersion: string,
): Promise<VersionNotes[]> {
	return call('update_changelogs', { path: dir, projectId, toVersion })
}

export function convertSearch(
	dir: string,
	projectId: string,
	target: ProviderId,
): Promise<ConvertCandidate | null> {
	return call('convert_search', { path: dir, projectId, target })
}

export function convertLookup(
	dir: string,
	projectId: string,
	target: ProviderId,
	query: string,
): Promise<ConvertCandidate> {
	return call('convert_lookup', { path: dir, projectId, target, query })
}

export function enrich(items: { id: string; provider: ProviderId }[]): Promise<ModMeta[]> {
	return call('enrich_mods', { items })
}

export function versions(
	projectId: string,
	provider: ProviderId,
	minecraft: string,
	loader: string | null,
): Promise<VersionInfo[]> {
	return call('mod_versions', { projectId, provider, minecraft, loader })
}

export function loaderVersions(loader: string, minecraft: string): Promise<LoaderVersions> {
	return call('get_loader_versions', { loader, minecraft })
}

export function minecraftVersions(includeSnapshots: boolean): Promise<string[]> {
	return call('get_minecraft_versions', { includeSnapshots })
}

export interface SearchHit {
	project_id: string
	slug: string
	title: string
	description: string
	author: string
	downloads: number
	follows: number
	icon_url: string | null
	categories: string[]
	display_categories: string[]
	project_type: string
	client_side: string | null
	server_side: string | null
	versions: string[]
}

export type ProjectType = 'mod' | 'resourcepack' | 'shader'

export interface Manifest {
	name: string
	version: string
	minecraft: string
	loader: string
	loaderVersion: string | null
	channel: string
}

export interface SourceFile {
	id?: string | null
	url?: string | null
	path?: string | null
	pin?: string | null
	slug: string
	versionId: string
	versionNumber: string
	filename: string
	downloadUrl: string
	sha1: string | null
	sha512: string | null
	fileSize: number
}

export interface LockedMod {
	name: string
	projectId: string
	slug: string
	projectType: string
	preferred: string
	sources: Record<string, SourceFile>
	dependencyType: 'explicit' | 'dependency'
	dependents: string[]
	clientSide: string
	serverSide: string
	disabled: boolean
}

export function activeSource(mod: LockedMod): SourceFile | undefined {
	return mod.sources[mod.preferred]
}

export interface Lockfile {
	minecraft: string
	loader: string
	loaderVersion: string | null
	mods: LockedMod[]
}

export type Severity = 'error' | 'warning' | 'info' | 'ok'

export interface Fix {
	kind: 'remove' | 'add' | 'none'
	projectId: string
	slug: string
	name: string
	label: string
}

export interface Validation {
	id: string
	severity: Severity
	title: string
	detail: string
	projectId: string | null
	fix: Fix | null
}

export interface Health {
	ok: boolean
	errors: number
	warnings: number
	infos: number
	modCount: number
	dependencyCount: number
}

export interface PackResolved {
	manifest: Manifest
	lockfile: Lockfile
	validations: Validation[]
	health: Health
}

export interface PackState {
	dir: string
	manifest: Manifest
	lockfile: Lockfile | null
}

export interface DownloadReport {
	downloaded: number
	failed: string[]
	modsDir: string
}

export type ModChangeKind =
	| 'instance_only'
	| 'pack_only'
	| 'version_diff'
	| 'unknown'
	| 'local_changed'
	| 'local_only'
	| 'disabled'
export type FileChangeKind = 'changed' | 'new' | 'removed'
export type SyncDirection = 'pull' | 'push' | 'ignore'

export interface ModChange {
	kind: ModChangeKind
	name: string
	projectId: string | null
	slug: string | null
	provider: string | null
	projectType: string
	instanceVersionId: string | null
	instanceVersion: string | null
	packVersion: string | null
	filename: string | null
	relPath: string | null
	dependency: boolean
}

export interface FileChange {
	kind: FileChangeKind
	path: string
}

export interface EnvChange {
	packMinecraft: string
	packLoader: string
	packLoaderVersion: string | null
	instanceMinecraft: string | null
	instanceLoader: string | null
	instanceLoaderVersion: string | null
	writable: boolean
}

export interface SyncReport {
	bound: boolean
	instanceDir: string | null
	mods: ModChange[]
	files: FileChange[]
	env: EnvChange | null
	inSync: boolean
}

export interface SyncOp {
	target: 'mod' | 'file' | 'env'
	kind: string
	direction: SyncDirection
	path?: string | null
	projectId?: string | null
	slug?: string | null
	provider?: string | null
	name?: string | null
	instanceVersionId?: string | null
	filename?: string | null
	relPath?: string | null
	projectType?: string
}

export type PackEnv = 'common' | 'server' | 'client'

export interface GitChange {
	status: string
	path: string
	staged: boolean
	conflicted: boolean
}

export interface GitStatus {
	isRepo: boolean
	branch: string | null
	upstream: string | null
	detached: boolean
	ahead: number
	behind: number
	hasRemote: boolean
	clean: boolean
	conflicts: number
	changes: GitChange[]
}

export interface GitCommit {
	hash: string
	short: string
	subject: string
	author: string
	email: string
	relative: string
}

export interface Branch {
	name: string
	current: boolean
	remote: boolean
	upstream: string | null
	ahead: number
	behind: number
	gone: boolean
}

export interface Branches {
	current: string | null
	detached: boolean
	list: Branch[]
}

export interface Remote {
	name: string
	url: string
}

export interface Stash {
	reference: string
	message: string
	branch: string
	relative: string
}

export interface Tag {
	name: string
	subject: string
}

export type PullStrategy = 'ff' | 'merge' | 'rebase'

export interface ImpactChange {
	kind: string
	name: string
	projectType: string
	fromVersion: string | null
	toVersion: string | null
	dependency: boolean
}

export interface ImpactReport {
	changes: ImpactChange[]
	problems: string[]
}

export interface PackItemChange {
	kind: string
	name: string
	projectType: string
	slug: string
	fromVersion: string | null
	toVersion: string | null
	fromSide: string | null
	toSide: string | null
	fromProvider: string | null
	toProvider: string | null
	disabled: boolean
}

export interface PackEnvDiff {
	fromMinecraft: string
	toMinecraft: string
	fromLoader: string
	toLoader: string
	fromLoaderVersion: string | null
	toLoaderVersion: string | null
}

export interface PackFileChange {
	status: string
	path: string
}

export interface PackDiff {
	items: PackItemChange[]
	env: PackEnvDiff | null
	files: PackFileChange[]
}

export interface ProviderInfo {
	id: string
	displayName: string
	configured: boolean
}

export interface ConvertCandidate {
	id: string
	slug: string
	name: string
	provider: string
	iconUrl: string | null
	author: string
	authorIconUrl: string | null
	version: string
}

export interface ModMeta {
	projectId: string
	iconUrl: string | null
	author: string
	authorIconUrl: string | null
}

export interface VersionInfo {
	id: string
	versionNumber: string
	versionType: string
	datePublished: string
	gameVersions: string[]
}

export interface BulkCandidate {
	provider: string
	projectId: string
	slug: string
	name: string
	projectType: string
	iconUrl?: string | null
	raw: string
}

export interface BulkFailure {
	raw: string
	reason: string
}

export interface BulkLookup {
	found: BulkCandidate[]
	failed: BulkFailure[]
}

export interface DroppedMatch {
	provider: string
	projectId: string
	slug: string
	name: string
	versionId: string
	versionNumber: string
	iconUrl?: string | null
}

export interface DroppedFile {
	path: string
	filename: string
	projectType: string
	alreadyInPack: boolean
	matched?: DroppedMatch
}

export interface ModDep {
	id: string
	version?: string
	kind: string
}

export interface JarMeta {
	id?: string
	name?: string
	version?: string
	description?: string
	authors?: string[]
	loaders?: string[]
	dependencies?: ModDep[]
	homepage?: string
	minecraft?: string
	packFormat?: number
}

export interface LocalContent {
	relPath: string
	filename: string
	projectType: string
	size: number
	meta?: JarMeta
}

export type InstanceKind = 'local' | 'modpack' | 'server'

export interface DetectedInstance {
	launcher: string
	name: string
	gameDir: string
	minecraft: string | null
	loader: string | null
	loaderVersion: string | null
	kind: InstanceKind
	source: string | null
	packName: string | null
	packVersion: string | null
}

export interface FsEntry {
	name: string
	isDir: boolean
	size: number
}

export interface FileContent {
	text: string | null
	binary: boolean
	tooLarge: boolean
	size: number
}

export interface NbtNode {
	name: string
	tag: string
	value: string | null
	children: NbtNode[]
}

export interface FileMatch {
	path: string
	line: number
	text: string
}

export const LOADERS = ['vanilla', 'fabric', 'forge', 'quilt', 'neoforge'] as const
export type Loader = (typeof LOADERS)[number]

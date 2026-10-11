import { invoke } from '@tauri-apps/api/core'

export type ErrorCode = 'git-auth' | 'git-missing' | 'git-identity' | 'instance-missing'

const CODES: Record<string, ErrorCode> = {
	GIT_AUTH: 'git-auth',
	GIT_MISSING: 'git-missing',
	GIT_IDENTITY: 'git-identity',
	MISSING_INSTANCE: 'instance-missing',
}

class CommandError extends Error {
	readonly code: ErrorCode | null

	constructor(message: string, code: ErrorCode | null) {
		super(message)
		this.code = code
	}
}

function toError(raw: unknown): CommandError {
	if (raw instanceof CommandError) return raw
	const text = typeof raw === 'string' ? raw : raw instanceof Error ? raw.message : String(raw)
	const match = /^([A-Z_]+):([\s\S]*)$/.exec(text)
	const code = match ? CODES[match[1]] : undefined
	return code ? new CommandError(match![2].trim(), code) : new CommandError(text, null)
}

export async function call<T>(command: string, args: Record<string, unknown> = {}): Promise<T> {
	try {
		return await invoke<T>(command, args)
	} catch (raw) {
		throw toError(raw)
	}
}

export function errorText(error: unknown): string {
	return toError(error).message
}

export function errorCode(error: unknown): ErrorCode | null {
	return toError(error).code
}

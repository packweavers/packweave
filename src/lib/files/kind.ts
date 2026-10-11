import { extension } from '@/lib/util/path'

import { langOf } from './highlight'

export type FileKind = 'image' | 'nbt' | 'markdown' | 'config' | 'code'

const IMAGES = ['png', 'jpg', 'jpeg', 'gif', 'webp', 'bmp']
const NBT = ['dat', 'nbt', 'dat_old', 'schematic', 'litematic']

export function fileKind(rel: string, text?: string): FileKind {
	const ext = extension(rel)
	if (IMAGES.includes(ext)) return 'image'
	if (NBT.includes(ext)) return 'nbt'
	if (ext === 'md' || ext === 'markdown') return 'markdown'
	const lang = langOf(rel, text)
	if (lang === 'json' || lang === 'json5' || lang === 'toml' || lang === 'properties') {
		return 'config'
	}
	return 'code'
}

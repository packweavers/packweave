import { DEFAULT_OPTIONS, type ChangelogOptions } from '@/lib/content/changelog'
import type { Channel } from '@/lib/tauri/types'

class ReleaseDraft {
	version = $state('')
	channel = $state<Channel>('release')
	notes = $state('')
	changelogText = $state('')
	changelogEdited = $state(false)
	options = $state<ChangelogOptions>({ ...DEFAULT_OPTIONS })
	range = $state<'release' | 'all'>('release')

	reset(): void {
		this.version = ''
		this.channel = 'release'
		this.notes = ''
		this.changelogText = ''
		this.changelogEdited = false
		this.options = { ...DEFAULT_OPTIONS }
		this.range = 'release'
	}
}

export const release = new ReleaseDraft()

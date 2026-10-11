import type { ContentFilter } from '@/lib/content/rows'
import type { ProjectType } from '@/lib/tauri/types'

class ContentView {
	filter = $state<Record<ProjectType, ContentFilter>>({
		mod: 'all',
		resourcepack: 'all',
		shader: 'all',
		datapack: 'all',
	})
	query = $state<Record<ProjectType, string>>({
		mod: '',
		resourcepack: '',
		shader: '',
		datapack: '',
	})

	show(type: ProjectType, filter: ContentFilter): void {
		this.filter[type] = filter
	}
}

export const contentView = new ContentView()

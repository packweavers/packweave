import { errorText } from '@/lib/tauri/call'
import { search } from '@/lib/tauri/catalog'
import type { CatalogHit, ProjectType, ProviderId } from '@/lib/tauri/types'

import { ws } from './workspace.svelte'

const PAGE = 30

class CatalogState {
	provider = $state<ProviderId>('modrinth')
	type = $state<ProjectType>('mod')
	query = $state('')
	results = $state.raw<CatalogHit[]>([])
	selected = $state.raw<CatalogHit | null>(null)
	loading = $state(false)
	error = $state<string | null>(null)
	more = $state(false)
	#request = 0

	async run(append = false): Promise<void> {
		const manifest = ws.manifest
		if (!manifest || this.provider === 'url') return
		const request = ++this.#request
		this.loading = true
		this.error = null
		try {
			const hits = await search({
				provider: this.provider,
				query: this.query,
				minecraft: manifest.minecraft,
				loader: manifest.loader,
				projectType: this.type,
				offset: append ? this.results.length : 0,
				limit: PAGE,
			})
			if (request !== this.#request) return
			this.results = append ? [...this.results, ...hits] : hits
			this.more = hits.length === PAGE
			if (!append) this.selected = hits[0] ?? null
		} catch (error) {
			if (request === this.#request) this.error = errorText(error)
		} finally {
			if (request === this.#request) this.loading = false
		}
	}
}

export const catalog = new CatalogState()

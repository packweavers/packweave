class Selection {
	byDoc = $state<Record<string, string[]>>({})

	of(docId: string | null | undefined): string[] {
		return docId ? (this.byDoc[docId] ?? []) : []
	}

	set(docId: string, keys: string[]): void {
		this.byDoc[docId] = keys
	}

	reset(): void {
		this.byDoc = {}
	}
}

export const selection = new Selection()

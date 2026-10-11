import type { Component, ComponentProps } from 'svelte'

type AnyComponent = Component<any>
type ResultOf<C extends AnyComponent> =
	ComponentProps<C> extends { close: (result?: infer R) => void } ? R : never

interface OpenDialog {
	component: AnyComponent
	props: Record<string, unknown>
	resolve: (value: unknown) => void
}

class Dialogs {
	current = $state.raw<OpenDialog | null>(null)

	open<C extends AnyComponent>(
		component: C,
		props: Omit<ComponentProps<C>, 'close'>,
	): Promise<ResultOf<C> | undefined> {
		this.current?.resolve(undefined)
		return new Promise((resolve) => {
			this.current = {
				component,
				props: props as Record<string, unknown>,
				resolve: resolve as (value: unknown) => void,
			}
		})
	}

	close(result?: unknown): void {
		const open = this.current
		this.current = null
		open?.resolve(result)
	}
}

export const dialogs = new Dialogs()

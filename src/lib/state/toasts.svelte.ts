import { errorText } from '@/lib/tauri/call'

export type ToastTone = 'done' | 'error' | 'info'

export interface ToastAction {
	label: string
	command?: string
	run: () => void
}

export interface Toast {
	id: number
	tone: ToastTone
	text: string
	actions: ToastAction[]
}

const LIFETIME = 6000

class Toasts {
	list = $state<Toast[]>([])
	#next = 1
	#timers = new Map<number, ReturnType<typeof setTimeout>>()

	show(text: string, actions: ToastAction[] = [], tone: ToastTone = 'done'): number {
		const id = this.#next++
		this.list = [...this.list.slice(-3), { id, tone, text, actions }]
		this.#arm(id)
		return id
	}

	info(text: string, actions: ToastAction[] = []): number {
		return this.show(text, actions, 'info')
	}

	error(error: unknown, actions: ToastAction[] = []): number {
		return this.show(errorText(error), actions, 'error')
	}

	dismiss(id: number): void {
		const timer = this.#timers.get(id)
		if (timer) clearTimeout(timer)
		this.#timers.delete(id)
		this.list = this.list.filter((t) => t.id !== id)
	}

	hold(id: number): void {
		const timer = this.#timers.get(id)
		if (timer) clearTimeout(timer)
		this.#timers.delete(id)
	}

	release(id: number): void {
		if (this.list.some((t) => t.id === id)) this.#arm(id)
	}

	#arm(id: number): void {
		this.#timers.set(
			id,
			setTimeout(() => this.dismiss(id), LIFETIME),
		)
	}
}

export const toasts = new Toasts()

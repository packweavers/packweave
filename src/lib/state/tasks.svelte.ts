import { errorText } from '@/lib/tauri/call'

export interface Task {
	id: number
	label: string
	started: number
}

export type LogTone = 'info' | 'done' | 'error'

export interface LogEntry {
	id: number
	time: number
	text: string
	tone: LogTone
}

const LOG_LIMIT = 500

class Tasks {
	running = $state<Task[]>([])
	log = $state<LogEntry[]>([])
	failures = $state(0)
	#next = 1

	get current(): Task | null {
		return this.running[0] ?? null
	}

	async run<T>(label: string, work: () => Promise<T>): Promise<T> {
		const task = { id: this.#next++, label, started: Date.now() }
		this.running = [...this.running, task]
		this.note(label, 'info')
		try {
			const result = await work()
			this.note(`${label}: done`, 'done')
			return result
		} catch (error) {
			this.note(`${label}: ${errorText(error)}`, 'error')
			this.failures++
			throw error
		} finally {
			this.running = this.running.filter((t) => t.id !== task.id)
		}
	}

	note(text: string, tone: LogTone = 'info'): void {
		this.log = [
			...this.log.slice(-(LOG_LIMIT - 1)),
			{ id: this.#next++, time: Date.now(), text, tone },
		]
	}

	fail(label: string, error: unknown): void {
		const text = `${label}: ${errorText(error)}`
		if (this.log.at(-1)?.text === text) return
		this.note(text, 'error')
		this.failures++
	}

	or<T>(label: string, fallback: T): (error: unknown) => T {
		return (error) => {
			this.fail(label, error)
			return fallback
		}
	}

	clear(): void {
		this.log = []
		this.failures = 0
	}
}

export const tasks = new Tasks()

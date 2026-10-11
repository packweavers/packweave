import type { GraphCommit } from '@/lib/tauri/types'

export interface GraphRow {
	commit: GraphCommit
	lane: number
	fromAbove: boolean
	through: number[]
	merging: number[]
	outgoing: number[]
	width: number
}

export function layoutGraph(commits: GraphCommit[]): GraphRow[] {
	const lanes: (string | null)[] = []
	const rows: GraphRow[] = []
	const freeLane = () => {
		const empty = lanes.indexOf(null)
		if (empty >= 0) return empty
		lanes.push(null)
		return lanes.length - 1
	}
	for (const commit of commits) {
		let lane = lanes.indexOf(commit.hash)
		const fromAbove = lane >= 0
		if (lane < 0) lane = freeLane()
		const merging: number[] = []
		const through: number[] = []
		lanes.forEach((hash, i) => {
			if (hash === null || i === lane) return
			if (hash === commit.hash) {
				merging.push(i)
			} else {
				through.push(i)
			}
		})
		for (const i of merging) lanes[i] = null
		lanes[lane] = null
		const outgoing: number[] = []
		const [first, ...rest] = commit.parents
		if (first) {
			lanes[lane] = first
			outgoing.push(lane)
		}
		for (const parent of rest) {
			let target = lanes.indexOf(parent)
			if (target < 0) {
				target = freeLane()
				lanes[target] = parent
			}
			outgoing.push(target)
		}
		while (lanes.length > 0 && lanes[lanes.length - 1] === null) lanes.pop()
		const width = Math.max(lane, ...through, ...merging, ...outgoing) + 1
		rows.push({ commit, lane, fromAbove, through, merging, outgoing, width })
	}
	return rows
}

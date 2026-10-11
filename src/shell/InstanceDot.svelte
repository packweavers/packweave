<script lang="ts">
	import type { InstanceStatus } from '@/lib/state/instance.svelte'

	interface Props {
		status: InstanceStatus
	}

	let { status }: Props = $props()

	const tone = $derived(
		{
			none: 'none',
			unknown: 'none',
			missing: 'error',
			running: 'running',
			'in-sync': 'ok',
			push: 'push',
			pull: 'pull',
			both: 'push',
		}[status],
	)
</script>

<span class="dot {tone}" aria-hidden="true"></span>

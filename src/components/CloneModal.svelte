<script lang="ts">
	import Modal from './ui/Modal.svelte'
	import ButtonStyled from './ui/ButtonStyled.svelte'
	import { store } from '../lib/store.svelte'
	import { autofocus } from '../lib/autofocus'

	let { onclose }: { onclose?: () => void } = $props()

	let url = $state('')

	async function clone() {
		if (!url.trim()) return
		const ok = await store.clonePack(url)
		if (ok) onclose?.()
	}
</script>

<Modal title="Clone a pack" onclose={() => onclose?.()}>
	<input
		bind:value={url}
		class="w-full bg-bg-inset border border-divider rounded-md px-3 h-9 text-sm text-contrast focus:border-brand outline-none mb-[1.1rem]"
		placeholder="https://github.com/you/pack.git"
		spellcheck="false"
		use:autofocus
		onkeydown={(e) => e.key === 'Enter' && clone()}
	/>
	<div class="flex justify-end gap-[0.6rem]">
		<ButtonStyled type="transparent" disabled={store.busy} onclick={() => onclose?.()}>
			Cancel
		</ButtonStyled>
		<ButtonStyled color="brand" disabled={!url.trim() || store.busy} onclick={clone}>
			Choose folder & clone
		</ButtonStyled>
	</div>
</Modal>

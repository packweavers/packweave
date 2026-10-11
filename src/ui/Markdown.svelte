<script lang="ts">
	import { renderHtml, renderMarkdown } from '@/lib/files/markdown'
	import { openExternal } from '@/lib/tauri/windows'

	interface Props {
		source: string
		html?: boolean
	}

	let { source, html = false }: Props = $props()

	const rendered = $derived(html ? renderHtml(source) : renderMarkdown(source))

	function onclick(event: MouseEvent): void {
		const link = (event.target as HTMLElement).closest('a')
		if (!link) return
		event.preventDefault()
		const href = link.getAttribute('href') ?? ''
		if (/^https?:\/\//i.test(href)) void openExternal(href)
	}
</script>

<div class="markdown" role="document" {onclick}>
	{@html rendered}
</div>

<style>
	.markdown {
		font-size: 13px;
		line-height: 1.55;
		user-select: text;
		-webkit-user-select: text;
		overflow-wrap: anywhere;
	}

	.markdown :global(:is(h1, h2, h3, h4, h5, h6)) {
		margin: 18px 0 8px;
		font-weight: 600;
	}

	.markdown :global(h1) {
		font-size: 20px;
	}

	.markdown :global(h2) {
		font-size: 16px;
	}

	.markdown :global(h3) {
		font-size: 14px;
	}

	.markdown :global(p) {
		margin: 0 0 10px;
	}

	.markdown :global(ul),
	.markdown :global(ol) {
		margin: 0 0 10px;
		padding-left: 20px;
	}

	.markdown :global(code) {
		padding: 0 4px;
		background: var(--band);
		font: 12px var(--font-mono);
	}

	.markdown :global(pre) {
		margin: 0 0 10px;
		padding: 10px 12px;
		background: var(--band);
		overflow: auto;
	}

	.markdown :global(pre code) {
		padding: 0;
		background: none;
	}

	.markdown :global(blockquote) {
		margin: 0 0 10px;
		padding-left: 12px;
		box-shadow: inset 2px 0 0 var(--edge);
	}

	.markdown :global(img) {
		max-width: 100%;
		height: auto;
	}

	.markdown :global(a) {
		color: var(--blue);
		text-decoration: none;
	}

	.markdown :global(a:hover) {
		text-decoration: underline;
	}

	.markdown :global(li > p) {
		margin: 0;
	}

	.markdown :global(li > :is(ul, ol)) {
		margin: 2px 0;
	}

	.markdown :global(input[type='checkbox']) {
		margin-right: 6px;
		vertical-align: -2px;
	}

	.markdown :global(table) {
		display: block;
		max-width: 100%;
		overflow-x: auto;
		border-collapse: collapse;
		margin: 0 0 10px;
	}

	.markdown :global(th),
	.markdown :global(td) {
		border: 1px solid var(--edge);
		padding: 4px 8px;
		text-align: left;
		vertical-align: top;
	}

	.markdown :global(th) {
		font-weight: 600;
		background: var(--band);
	}

	.markdown :global(th[align='center']),
	.markdown :global(td[align='center']) {
		text-align: center;
	}

	.markdown :global(th[align='right']),
	.markdown :global(td[align='right']) {
		text-align: right;
	}

	.markdown :global(details) {
		margin: 0 0 10px;
	}

	.markdown :global(summary) {
		font-weight: 600;
	}

	.markdown :global(h4),
	.markdown :global(h5),
	.markdown :global(h6) {
		font-size: 13px;
	}

	.markdown :global(hr) {
		border: 0;
		border-top: 1px solid var(--line);
		margin: 14px 0;
	}

	.markdown > :global(:first-child) {
		margin-top: 0;
	}
</style>

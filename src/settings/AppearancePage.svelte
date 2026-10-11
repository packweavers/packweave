<script lang="ts">
	import { app } from '@/lib/state/app.svelte'
	import { EDITOR_SIZES, UI_SCALES, type ThemePref } from '@/lib/state/prefs'
	import Segmented from '@/ui/Segmented.svelte'
</script>

<div class="band bar">Appearance</div>
<div class="setting">
	<span class="key">Theme</span>
	<Segmented
		options={[
			{ value: 'system' as ThemePref, label: 'System' },
			{ value: 'light' as ThemePref, label: 'Light' },
			{ value: 'dark' as ThemePref, label: 'Dark' },
		]}
		value={app.prefs.theme}
		onchange={(theme) => app.setTheme(theme)}
	/>
</div>
<div class="setting">
	<span class="key">Interface size</span>
	<Segmented
		options={UI_SCALES.map((scale) => ({
			value: String(scale),
			label: `${Math.round(scale * 100)}%`,
		}))}
		value={String(app.prefs.uiScale)}
		onchange={(scale) => app.setScale(Number(scale))}
	/>
</div>
<div class="setting">
	<span class="key">Editor font size</span>
	<Segmented
		options={EDITOR_SIZES.map((size) => ({ value: String(size), label: String(size) }))}
		value={String(app.prefs.editorSize)}
		onchange={(size) => app.setEditorSize(Number(size))}
	/>
</div>

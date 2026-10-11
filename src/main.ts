import '@fontsource/instrument-sans/400.css'
import '@fontsource/instrument-sans/400-italic.css'
import '@fontsource/instrument-sans/500.css'
import '@fontsource/instrument-sans/600.css'
import '@fontsource/jetbrains-mono/400.css'
import '@fontsource/jetbrains-mono/500.css'
import '@fontsource/jetbrains-mono/600.css'
import './styles/tokens.css'
import './styles/base.css'
import './styles/controls.css'
import './styles/surfaces.css'
import './styles/overlays.css'
import './styles/code.css'

import { mount } from 'svelte'

import App from './App.svelte'
import { registerAll } from './lib/commands'
import { app } from './lib/state/app.svelte'
import { followScale, followTheme } from './lib/state/theme.svelte'
import SettingsApp from './settings/SettingsApp.svelte'

async function boot(): Promise<void> {
	await app.load()
	followTheme()
	followScale()
	registerAll()
	const settings = new URLSearchParams(location.search).get('window') === 'settings'
	mount(settings ? SettingsApp : App, { target: document.getElementById('app')! })
}

void boot()

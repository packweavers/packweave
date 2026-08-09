import { mount } from 'svelte'
import './app.css'
import App from './App.svelte'
import { api } from './api'
import { store } from './lib/store.svelte'

async function boot() {
	let prefs: Record<string, unknown> = {}
	try {
		prefs = JSON.parse(await api.readPrefs()) as Record<string, unknown>
	} catch {
		prefs = {}
	}
	if (prefs.theme === 'light') document.documentElement.classList.add('theme-light')
	else if (prefs.theme === 'dark') document.documentElement.classList.add('theme-dark')

	store.hydratePrefs(prefs)
	mount(App, { target: document.getElementById('app')! })
}

void boot()

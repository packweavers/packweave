import { s, notify, dismiss } from './store/state.svelte'
import * as prefs from './store/prefs'
import * as resolve from './store/resolve'
import * as content from './store/content'
import * as git from './store/git'
import * as instance from './store/instance'
import * as lifecycle from './store/lifecycle'
import * as publish from './store/publish'

export type {
	ToastKind,
	Toast,
	RecentPack,
	ThemePref,
	PackView,
	GitProvider,
} from './store/state.svelte'

export const store = Object.assign(s, {
	notify,
	dismiss,
	...prefs,
	...resolve,
	...content,
	...git,
	...instance,
	...lifecycle,
	...publish,
})

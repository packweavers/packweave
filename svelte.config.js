import { vitePreprocess } from '@sveltejs/vite-plugin-svelte'

const SUPPRESSED_A11Y = new Set([
  'a11y_no_noninteractive_element_to_interactive_role',
  'a11y_no_static_element_interactions',
  'a11y_no_noninteractive_element_interactions',
  'a11y_click_events_have_key_events',
])

export default {
  preprocess: vitePreprocess(),
  compilerOptions: {
    warningFilter: (warning) => !SUPPRESSED_A11Y.has(warning.code),
  },
}

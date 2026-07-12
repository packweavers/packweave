import type { Config } from 'tailwindcss'

export default {
	content: ['./index.html', './src/**/*.{svelte,ts}'],
	theme: {
		extend: {
			colors: {
				brand: {
					DEFAULT: 'var(--color-brand)',
					hover: 'var(--color-brand-hover)',
					highlight: 'var(--color-brand-highlight)',
				},
				'on-brand': 'var(--color-on-brand)',
				bg: {
					DEFAULT: 'var(--color-bg)',
					raised: 'var(--color-raised-bg)',
					'super-raised': 'var(--color-super-raised-bg)',
					inset: 'var(--color-inset-bg)',
				},
				surface: {
					1: 'var(--color-surface-1)',
					2: 'var(--color-surface-2)',
					3: 'var(--color-surface-3)',
					4: 'var(--color-surface-4)',
					5: 'var(--color-surface-5)',
				},
				button: {
					bg: 'var(--color-button-bg)',
					'bg-hover': 'var(--color-button-bg-hover)',
				},
				sidebar: 'var(--color-sidebar-bg)',
				chrome: 'var(--color-chrome-bg)',
				body: 'var(--color-base)',
				secondary: 'var(--color-secondary)',
				contrast: 'var(--color-contrast)',
				divider: {
					DEFAULT: 'var(--color-divider)',
					dark: 'var(--color-divider-dark)',
				},
				red: 'var(--color-red)',
				orange: 'var(--color-orange)',
				green: 'var(--color-green)',
				blue: 'var(--color-blue)',
				purple: 'var(--color-purple)',
				link: 'var(--color-link)',
			},
			borderRadius: {
				sm: 'var(--radius-sm)',
				DEFAULT: 'var(--radius-md)',
				md: 'var(--radius-md)',
				lg: 'var(--radius-lg)',
				xl: 'var(--radius-xl)',
				max: 'var(--radius-max)',
			},
			boxShadow: {
				card: 'var(--shadow-card)',
				raised: 'var(--shadow-raised)',
				floating: 'var(--shadow-floating)',
			},
			fontFamily: {
				sans: [
					'-apple-system',
					'BlinkMacSystemFont',
					'"SF Pro Text"',
					'"SF Pro Display"',
					'system-ui',
					'sans-serif',
				],
				mono: ['"SF Mono"', 'ui-monospace', 'Menlo', 'monospace'],
			},
		},
	},
	plugins: [],
} satisfies Config

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
			fontSize: {
				micro: '0.65rem',
				tiny: '0.72rem',
				small: '0.78rem',
				regular: '0.85rem',
				large: '0.92rem',
				title: '1.1rem',
				display: '1.28rem',
				hero: '1.5rem',
			},
			spacing: {
				hair: '0.05rem',
				'2xs': '0.15rem',
				sm: '0.35rem',
				md: '0.42rem',
				lg: '0.55rem',
				'4xl': '1.15rem',
				'5xl': '1.7rem',
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

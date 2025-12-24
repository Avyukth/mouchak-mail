import { fontFamily } from 'tailwindcss/defaultTheme';

/** @type {import('tailwindcss').Config} */
export default {
	darkMode: ['class'],
	content: ['./src/**/*.{html,js,svelte,ts}'],
	safelist: ['dark'],
	theme: {
		container: {
			center: true,
			padding: '2rem',
			screens: {
				'2xl': '1400px'
			}
		},
		extend: {
			colors: {
				border: 'hsl(var(--border) / <alpha-value>)',
				input: 'hsl(var(--input) / <alpha-value>)',
				ring: 'hsl(var(--ring) / <alpha-value>)',
				background: 'hsl(var(--background) / <alpha-value>)',
				foreground: 'hsl(var(--foreground) / <alpha-value>)',
				primary: {
					DEFAULT: 'hsl(var(--primary) / <alpha-value>)',
					foreground: 'hsl(var(--primary-foreground) / <alpha-value>)',
					50: '#eef2ff',
					100: '#e0e7ff',
					200: '#c7d2fe',
					300: '#a5b4fc',
					400: '#818cf8',
					500: '#6366f1',
					600: '#4f46e5',
					700: '#4338ca',
					800: '#3730a3',
					900: '#312e81',
					950: '#1e1b4b'
				},
				secondary: {
					DEFAULT: 'hsl(var(--secondary) / <alpha-value>)',
					foreground: 'hsl(var(--secondary-foreground) / <alpha-value>)'
				},
				destructive: {
					DEFAULT: 'hsl(var(--destructive) / <alpha-value>)',
					foreground: 'hsl(var(--destructive-foreground) / <alpha-value>)'
				},
				muted: {
					DEFAULT: 'hsl(var(--muted) / <alpha-value>)',
					foreground: 'hsl(var(--muted-foreground) / <alpha-value>)'
				},
				accent: {
					DEFAULT: 'hsl(var(--accent) / <alpha-value>)',
					foreground: 'hsl(var(--accent-foreground) / <alpha-value>)'
				},
				popover: {
					DEFAULT: 'hsl(var(--popover) / <alpha-value>)',
					foreground: 'hsl(var(--popover-foreground) / <alpha-value>)'
				},
				card: {
					DEFAULT: 'hsl(var(--card) / <alpha-value>)',
					foreground: 'hsl(var(--card-foreground) / <alpha-value>)'
				},
				// Status colors (semantic)
				status: {
					online: 'hsl(var(--status-online) / <alpha-value>)',
					offline: 'hsl(var(--status-offline) / <alpha-value>)',
					pending: 'hsl(var(--status-pending) / <alpha-value>)',
					idle: 'hsl(var(--status-idle) / <alpha-value>)'
				},
				// Semantic colors
				success: {
					DEFAULT: '#10b981',
					50: '#f0fdf4',
					100: '#dcfce7',
					200: '#bbf7d0',
					300: '#86efac',
					400: '#4ade80',
					500: '#10b981',
					600: '#059669',
					700: '#047857',
					800: '#065f46',
					900: '#064e3b'
				},
				warning: {
					DEFAULT: '#f59e0b',
					50: '#fffbeb',
					100: '#fef3c7',
					200: '#fde68a',
					300: '#fcd34d',
					400: '#fbbf24',
					500: '#f59e0b',
					600: '#d97706',
					700: '#b45309',
					800: '#92400e',
					900: '#78350f'
				},
				danger: {
					DEFAULT: '#ef4444',
					50: '#fef2f2',
					100: '#fee2e2',
					200: '#fecaca',
					300: '#fca5a5',
					400: '#f87171',
					500: '#ef4444',
					600: '#dc2626',
					700: '#b91c1c',
					800: '#991b1b',
					900: '#7f1d1d'
				},
				success: {
					DEFAULT: 'hsl(var(--success) / <alpha-value>)',
					foreground: 'hsl(var(--success-foreground) / <alpha-value>)'
				},
				warning: {
					DEFAULT: 'hsl(var(--warning) / <alpha-value>)',
					foreground: 'hsl(var(--warning-foreground) / <alpha-value>)'
				},
				info: {
					DEFAULT: 'hsl(var(--info) / <alpha-value>)',
					foreground: 'hsl(var(--info-foreground) / <alpha-value>)'
				},
				shimmer: 'hsl(var(--shimmer-color) / <alpha-value>)'
			},
			// Spacing aliases for 8px grid
			spacing: {
				'touch': 'var(--touch-target-min)'
			},
			// Min height for touch targets
			minHeight: {
				'touch': 'var(--touch-target-min)'
			},
			minWidth: {
				'touch': 'var(--touch-target-min)'
			},
			borderRadius: {
				lg: 'var(--radius)',
				md: 'calc(var(--radius) - 2px)',
				sm: 'calc(var(--radius) - 4px)'
			},
			fontFamily: {
				sans: [...fontFamily.sans]
			},
			fontSize: {
				'2xs': ['0.625rem', { lineHeight: '0.875rem' }]
			},
			boxShadow: {
				'sm': 'var(--shadow-sm)',
				'md': 'var(--shadow-md)',
				'lg': 'var(--shadow-lg)',
				'xl': 'var(--shadow-xl)',
				'focus': 'var(--shadow-focus)'
			},
			keyframes: {
				// shadcn/bits-ui keyframes
				'accordion-down': {
					from: { height: '0' },
					to: { height: 'var(--bits-accordion-content-height)' }
				},
				'accordion-up': {
					from: { height: 'var(--bits-accordion-content-height)' },
					to: { height: '0' }
				},
				'caret-blink': {
					'0%,70%,100%': { opacity: '1' },
					'20%,50%': { opacity: '0' }
				},
				// Entrance animations
				'fade-in': {
					from: { opacity: '0' },
					to: { opacity: '1' }
				},
				'fade-in-up': {
					from: { opacity: '0', transform: 'translateY(10px)' },
					to: { opacity: '1', transform: 'translateY(0)' }
				},
				'fade-in-down': {
					from: { opacity: '0', transform: 'translateY(-10px)' },
					to: { opacity: '1', transform: 'translateY(0)' }
				},
				'slide-in-right': {
					from: { opacity: '0', transform: 'translateX(20px)' },
					to: { opacity: '1', transform: 'translateX(0)' }
				},
				'slide-in-left': {
					from: { opacity: '0', transform: 'translateX(-20px)' },
					to: { opacity: '1', transform: 'translateX(0)' }
				},
				'scale-in': {
					from: { opacity: '0', transform: 'scale(0.95)' },
					to: { opacity: '1', transform: 'scale(1)' }
				},
				'blur-fade-in': {
					from: { opacity: '0', filter: 'blur(4px)' },
					to: { opacity: '1', filter: 'blur(0)' }
				}
			},
			animation: {
				// shadcn/bits-ui animations
				'accordion-down': 'accordion-down 0.2s ease-out',
				'accordion-up': 'accordion-up 0.2s ease-out',
				'caret-blink': 'caret-blink 1.25s ease-out infinite',
				// Entrance animations (using CSS variable timing)
				'fade-in': 'fade-in var(--duration-normal, 200ms) var(--ease-out, ease-out)',
				'fade-in-up': 'fade-in-up var(--duration-normal, 200ms) var(--ease-out, ease-out)',
				'fade-in-down': 'fade-in-down var(--duration-normal, 200ms) var(--ease-out, ease-out)',
				'slide-in-right': 'slide-in-right var(--duration-normal, 200ms) var(--ease-out, ease-out)',
				'slide-in-left': 'slide-in-left var(--duration-normal, 200ms) var(--ease-out, ease-out)',
				'scale-in': 'scale-in var(--duration-fast, 150ms) var(--ease-out, ease-out)',
				'blur-fade-in': 'blur-fade-in var(--duration-slow, 300ms) var(--ease-out, ease-out)',
				// Quick variants
				'fade-in-fast': 'fade-in var(--duration-fast, 150ms) var(--ease-out, ease-out)',
				'scale-in-fast': 'scale-in var(--duration-fast, 150ms) var(--ease-out, ease-out)'
			},
			transitionDelay: {
				'0': '0ms',
				'50': '50ms',
				'100': '100ms',
				'150': '150ms',
				'200': '200ms',
				'300': '300ms',
				'400': '400ms',
				'500': '500ms'
			}
		}
	},
	plugins: []
};

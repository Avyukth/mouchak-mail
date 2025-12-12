/** @type {import('tailwindcss').Config} */
module.exports = {
    content: ['./src/**/*.rs', './index.html'],
    darkMode: 'class',
    theme: {
        extend: {
            colors: {
                // Warm neutrals
                cream: {
                    50: '#FDFCFA',
                    100: '#FAF8F5',
                    200: '#F5F3EF',
                    300: '#E8E6E1',
                    400: '#D4D2CC',
                    500: '#A8A69D',
                },
                charcoal: {
                    50: '#F5F5F4',
                    100: '#E7E7E5',
                    200: '#D4D4D2',
                    300: '#A3A39E',
                    400: '#6B6A64',
                    500: '#4D4D46',
                    600: '#3D3D38',
                    700: '#2E2E2A',
                    800: '#1A1A18',
                    900: '#0F0F0E',
                },
                // Accent colors
                amber: {
                    50: '#FFFBEB',
                    100: '#FEF3C7',
                    200: '#FDE68A',
                    300: '#FCD34D',
                    400: '#FBBF24',
                    500: '#F59E0B',
                    600: '#D97706',
                    700: '#B45309',
                    800: '#92400E',
                    900: '#78350F',
                },
                teal: {
                    50: '#F0FDFA',
                    100: '#CCFBF1',
                    200: '#99F6E4',
                    300: '#5EEAD4',
                    400: '#2DD4BF',
                    500: '#14B8A6',
                    600: '#0D9488',
                    700: '#0F766E',
                    800: '#115E59',
                    900: '#134E4A',
                },
                violet: {
                    50: '#F5F3FF',
                    100: '#EDE9FE',
                    200: '#DDD6FE',
                    300: '#C4B5FD',
                    400: '#A78BFA',
                    500: '#8B5CF6',
                    600: '#7C3AED',
                    700: '#6D28D9',
                    800: '#5B21B6',
                    900: '#4C1D95',
                },
            },
            fontFamily: {
                display: ['Outfit', 'system-ui', 'sans-serif'],
                sans: ['Inter', 'system-ui', '-apple-system', 'sans-serif'],
                mono: ['JetBrains Mono', 'Fira Code', 'monospace'],
            },
            boxShadow: {
                'soft': '0 2px 8px -2px rgba(26, 26, 26, 0.08)',
                'medium': '0 4px 12px -4px rgba(26, 26, 26, 0.12)',
                'strong': '0 8px 24px -8px rgba(26, 26, 26, 0.16)',
                'glow-amber': '0 0 20px rgba(217, 119, 6, 0.15)',
                'glow-teal': '0 0 20px rgba(15, 118, 110, 0.15)',
            },
            animation: {
                'fade-in': 'fade-in 0.3s ease-out',
                'slide-up': 'slide-up 0.4s ease-out',
                'pulse-gentle': 'pulse-gentle 2s ease-in-out infinite',
            },
            keyframes: {
                'fade-in': {
                    '0%': { opacity: '0' },
                    '100%': { opacity: '1' },
                },
                'slide-up': {
                    '0%': { opacity: '0', transform: 'translateY(10px)' },
                    '100%': { opacity: '1', transform: 'translateY(0)' },
                },
                'pulse-gentle': {
                    '0%, 100%': { opacity: '1' },
                    '50%': { opacity: '0.7' },
                },
            },
            backdropBlur: {
                xs: '2px',
            },
            backgroundImage: {
                'gradient-radial': 'radial-gradient(var(--tw-gradient-stops))',
                'gradient-mesh': 'radial-gradient(at 40% 20%, rgba(217, 119, 6, 0.04) 0px, transparent 50%), radial-gradient(at 80% 0%, rgba(124, 58, 237, 0.03) 0px, transparent 50%), radial-gradient(at 0% 50%, rgba(15, 118, 110, 0.03) 0px, transparent 50%)',
            },
        },
    },
    plugins: [],
};

/** @type {import('tailwindcss').Config} */
export default {
  content: [
    './src/**/*.{html,js,svelte,ts}'
  ],
  theme: {
    extend: {
      colors: {
        black: '#18181B',
        surface: '#1F1F23',
        'surface-raised': '#27272B',
        gold: '#FFFFFF',
        'gold-dim': '#71717A',
        accent: '#6366F1',
        ivory: '#FAFAFA',
        'ivory-muted': '#A1A1AA',
        positive: '#4ADE80',
        warning: '#FBBF24',
        negative: '#F87171',
        border: '#303036'
      },
      fontFamily: {
        display: ['Inter', '-apple-system', 'sans-serif'],
        body: ['Inter', '-apple-system', 'sans-serif'],
        mono: ['JetBrains Mono', 'monospace']
      }
    }
  },
  plugins: []
};

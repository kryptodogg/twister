/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./index.html",
    "./src/**/*.{js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {
      colors: {
        'br-green':  'var(--br-green)',
        'br-tan':    'var(--br-tan)',
        'br-teal':   'var(--br-teal)',
        'br-purple': 'var(--br-purple)',
        'br-slate':  'var(--br-slate)',
        'surface':   'var(--surface)',
        'surface-card': 'var(--surface-card)',
        'surface-elevated': 'var(--surface-elevated)',
        'connected':    'var(--color-connected)',
        'disconnected': 'var(--color-disconnected)',
        'unwired':      'var(--color-unwired)',
        'rx':           'var(--color-rx)',
        'tx':           'var(--color-tx)',
        'config':       'var(--color-config)',
        'controller':   'var(--color-controller)',
      },
    },
  },
  plugins: [],
}

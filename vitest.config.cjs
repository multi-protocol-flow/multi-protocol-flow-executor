const path = require('path');

module.exports = {
  plugins: [],
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './src'),
    },
  },
  test: {
    globals: true,
    environment: 'jsdom',
    setupFiles: ['./src/test/setup.ts'],
    include: ['src/**/__tests__/**/*.{test,spec}.{js,jsx,ts,tsx}'],
    css: false,
  },
}

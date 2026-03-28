import { defineConfig } from 'vite';

export default defineConfig({
  base: process.env.DEMO_BASE_PATH || '/',
  build: {
    target: 'esnext',
  },
  server: {
    headers: {
      'Cross-Origin-Opener-Policy': 'same-origin',
      'Cross-Origin-Embedder-Policy': 'require-corp',
    },
    fs: {
      allow: ['..'],
    },
  },
});

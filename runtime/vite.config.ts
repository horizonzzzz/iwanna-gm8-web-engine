import { defineConfig } from 'vite';

export default defineConfig({
  server: {
    port: 4173
  },
  test: {
    environment: 'node',
    include: ['src/**/*.test.ts']
  }
});

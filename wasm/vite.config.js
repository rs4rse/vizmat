import { defineConfig } from 'vite';
import wasm from 'vite-plugin-wasm';

export default defineConfig({
  plugins: [wasm()],
  base: '/',
  optimizeDeps: {
    exclude: ['vizmat-core']
  }
});

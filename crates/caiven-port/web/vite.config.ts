import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import tailwindcss from '@tailwindcss/vite';
import path from "path";

export default defineConfig({
  plugins: [tailwindcss(), svelte()],
  resolve: {
    preserveSymlinks: true,
    alias: {
      $lib: path.resolve("./src/lib"),
    },
  },
  server: {
    proxy: {
      '/api': process.env.CAIVEN_E2E_API_TARGET ?? 'http://localhost:8080',
    },
  },
});

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
    // AdminShell is the only route behind a dynamic import() (App.svelte).
    // Without this, the first parallel e2e worker to hit an admin route
    // triggers a fresh dependency scan mid-run, invalidating chunks already
    // being served to other workers (Vite's "Outdated Optimize Dep" 504).
    // Warming it during server startup does that scan once, up front.
    warmup: {
      clientFiles: ['./src/pages/admin/AdminShell.svelte'],
    },
  },
});

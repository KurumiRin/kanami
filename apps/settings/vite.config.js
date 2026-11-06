import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";

// Tauri dev mode connects to a fixed port; the build goes to dist/ and src-tauri bundles it into the app
export default defineConfig({
  plugins: [svelte()],
  clearScreen: false,
  server: { port: 5173, strictPort: true },
  build: { outDir: "dist", target: "es2022", chunkSizeWarningLimit: 1500 },
});

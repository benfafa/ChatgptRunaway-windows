import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import { resolve } from "node:path";

// Tauri injects TAURI_DEV_HOST and friends. See:
// https://v2.tauri.app/start/frontend/vite/
const host = process.env.TAURI_DEV_HOST;

export default defineConfig({
  plugins: [react()],
  // The frontend lives in `web/` to keep the repository root clean.
  root: resolve(__dirname, "web"),
  // Vite's default build output goes to `web/dist`; tauri.conf.json expects
  // `../web/dist` relative to src-tauri.
  build: {
    outDir: resolve(__dirname, "web/dist"),
    emptyOutDir: true,
    target: ["chrome105", "safari13"],
    minify: !process.env.TAURI_ENV_DEBUG ? "esbuild" : false,
    sourcemap: !!process.env.TAURI_ENV_DEBUG,
  },
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? { protocol: "ws", host, port: 1421 }
      : undefined,
    watch: {
      // src-tauri is Rust; vite should not try to bundle it.
      ignored: ["**/src-tauri/**", "**/target/**"],
    },
  },
  envPrefix: ["VITE_", "TAURI_ENV_*"],
});

import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// Tauri expects a fixed port and fails if it is taken, rather than silently moving.
const DEV_PORT = 1420;

export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: {
    port: DEV_PORT,
    strictPort: true,
    // The Rust side rebuilds on its own; watching it here only causes double reloads.
    watch: { ignored: ["**/src-tauri/**"] },
  },
  build: {
    // WebKit on macOS 12 and WebView2 on Windows 10 both handle ES2021.
    target: "es2021",
    // MapLibre alone is about 800 kB. The app loads from disk, so one large
    // chunk costs nothing and splitting it would only add indirection.
    chunkSizeWarningLimit: 2000,
  },
});

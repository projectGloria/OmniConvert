import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";

// Tauri expects a fixed dev port; strictPort makes a port clash fail loudly
// instead of silently moving to 1421 while Tauri keeps loading 1420.
export default defineConfig({
  plugins: [react(), tailwindcss()],
  clearScreen: false, // keep Rust compiler output visible in the same terminal
  server: {
    port: 1420,
    strictPort: true,
    watch: { ignored: ["**/src-tauri/**"] }, // cargo's target/ churn would trigger endless HMR reloads
  },
  build: {
    target: "es2022", // WebView2 is evergreen Chromium; no need to down-level
    sourcemap: false,
  },
});

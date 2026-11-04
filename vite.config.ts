import { defineConfig, loadEnv } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";

// Two builds from one React tree:
//   default mode → Tauri desktop (platform/tauri)
//   --mode web   → GitHub Pages (platform/web: WASI engine in Web Workers)
// `@platform` is resolved here, so each bundle contains exactly one host adapter.
export default defineConfig(({ mode }) => {
  const web = mode === "web";
  // Root-relative id: Vite resolves "/src/…" against the project root (no node:path needed).
  const platformEntry = `/src/platform/${web ? "web" : "tauri"}/index.ts`;

  return {
    plugins: [react(), tailwindcss()],
    // Pages serves project sites from /<repo>/. Override with VITE_BASE for a custom domain.
    base: web ? (loadEnv(mode, ".", "VITE_").VITE_BASE ?? "/OmniConvert/") : "/",
    resolve: { alias: { "@platform": platformEntry } },
    clearScreen: false, // keep Rust compiler output visible in the same terminal
    server: {
      // Tauri expects a fixed dev port; strictPort makes a clash fail loudly instead of
      // silently moving to 1421 while Tauri keeps loading 1420. The web dev server uses 1430.
      port: web ? 1430 : 1420,
      strictPort: true,
      watch: { ignored: ["**/src-tauri/**", "**/target/**", "**/crates/**"] }, // cargo churn ≠ HMR
    },
    worker: { format: "es" }, // the engine worker is an ES module (import.meta.url, top-level imports)
    build: {
      // Desktop: WebView2 is evergreen Chromium. Web: module workers + OffscreenCanvas
      // need Chrome 80 / Firefox 114 / Safari 16.4-class browsers anyway.
      target: "es2022",
      outDir: web ? "dist-web" : "dist",
      sourcemap: false,
    },
  };
});

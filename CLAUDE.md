# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

# Project: OmniConvert (Tauri v2 Offline Universal File Converter for Windows)

Two editions from one codebase:
- **Desktop** (Tauri, Windows x64 `x86_64-pc-windows-msvc`): 100% offline, zero runtime network calls, every engine.
- **Web** (GitHub Pages, `--mode web`): the pure-Rust engines (image, data, archive) compiled to WASI and run in Web Workers. Video/audio/documents/PDF show "Desktop app only" for now.

`PLAN.md` holds the design decisions (D1–D10) and the scope cuts that were made on purpose
(no PDF→DOCX, no raster→SVG, ODS/XLS read-only, GZ/XZ single-stream). Read it before
adding routes or reversing one of those decisions.

## Commands

```bash
npm run sidecars          # one-time dev setup: fetch + SHA-verify ffmpeg/ffprobe/pandoc/typst/pdfium (pinned in scripts/sidecars.lock.json)
npm run tauri dev         # run the desktop app (Vite on :1420 + Rust)
npm run build:wasm        # build crates/web → src/platform/web/engine/*.wasm (needs `rustup target add wasm32-wasip1`)
npm run dev:web           # web edition on :1430/OmniConvert/ (run build:wasm first, and after any engine change)
npm run build:web         # build:wasm + tsc + vite build --mode web → dist-web/ (what CI deploys)
npm run verify            # the phase gate: typecheck + check:loc + clippy -D warnings + cargo test
npm run typecheck         # tsc --noEmit
npm run check:loc         # fails if any hand-written .ts/.tsx/.rs file > 200 lines
npm run lint:rs           # cargo clippy --workspace --all-targets -- -D warnings
npm run test:rs           # cargo test --workspace: unit tests + native e2e routes
npm run test:e2e          # e2e_routes incl. #[ignore]d sidecar routes (needs `npm run sidecars`)
npm run bindings          # regenerate src/types/generated/*.ts from Rust (ts-rs)
npm run tauri:build       # NSIS + MSI installers (merges tauri.bundle.conf.json)
```

Single test: `cargo test -p omniconvert-core <name_substring>` (engines/registry),
`cargo test -p omniconvert <name>` (desktop), or for e2e
`cargo test -p omniconvert --test e2e_routes <name> -- --include-ignored`.

Cargo is a **workspace at the repo root** (`crates/core`, `crates/web`, `src-tauri`; one
`Cargo.lock`, one `target/`, profiles in the root `Cargo.toml`). Run cargo from the root:
`.cargo/config.toml` sets `TS_RS_EXPORT_DIR=src/types/generated`, and Cargo finds config by
walking up from the cwd.

Deploy: `.github/workflows/pages.yml` builds `build:web` on every push to `main`
(Pages source must be set to "GitHub Actions"). `VITE_BASE` sets the Pages subpath.

## Architecture

**Crates.** `crates/core` (`omniconvert-core`): registry, pure-Rust engines (image/svg,
data, archive), `output` naming, `JobSettings`, `ProgressSink`. No Tauri/Tokio runtime/C
deps, so it builds for `wasm32-wasip1` (features `parallel` = rayon, `lossy-webp` =
libwebp are desktop-only defaults). `src-tauri` re-exports core modules at the old paths
(`crate::registry`, `crate::engines::{image,data,archive}`, `queue::output`) and owns the
queue, sidecar engines (media, document, pdfium) and GPU. Keep C-linked crates out of core.

**Rust is the single source of truth for formats (D1).** `crates/core/src/registry/` does
detection (`detect.rs`, content sniffing), the compatibility matrix (`matrix.rs`), shape
traits that gate targets (`shape.rs`, e.g. a plain `.gz` target only for single-entry
sources), and engine routing (`route.rs`: `route_for(src, TargetKind) -> EngineRoute`).
The frontend never hardcodes targets. It calls `probe_files` and renders `FileInfo.targets`.
New formats/routes go in the registry, then get an e2e route test. The web engine only
*subtracts* routes it can't run (`crates/web/src/caps.rs`): update it when a web engine lands.

**Generated types:** structs marked `#[derive(TS)] #[ts(export)]` are exported to
`src/types/generated/` by `npm run bindings`. Never hand-edit those files. Regenerate them
after changing any exported Rust type.

**Platform seam (frontend).** Components never import `@tauri-apps/*` or web-engine code;
they use `@platform` (interface: `src/platform/types.ts`), aliased by `vite.config.ts` to
`platform/tauri` (default) or `platform/web` (`--mode web`). tsconfig maps it to the tauri
adapter; both are type-checked since both live under `src/`. The contract is the desktop's:
string "paths" in (web: File-registry keys, `platform/web/files.ts`), `conversion://*` events out
(web: emitted by `platform/web/pool.ts` via an in-page bus).

**Web engine.** `crates/web` is a WASI *command* (`probe|convert|merge <json>`; JSON reply on
stdout, throttled progress lines on stderr). `platform/web/worker.ts` runs one fresh instance per
job via `@bjorn3/browser_wasi_shim`: inputs mounted lazily at `/in/<i>/<name>` (`BlobFile`,
FileReaderSync), outputs collected from an in-memory `/out`. `wasiFs.ts` patches the shim's
O(n²) file growth; keep that if upgrading the shim. Cancel = `worker.terminate()`. Browser
codecs fill the gaps: AVIF input is decoded via `createImageBitmap`, lossy WebP is encoded via
`OffscreenCanvas` (Rust emits PNG first; falls back to Rust's lossless WebP). Release profile
for WASM is `web` (`panic = "abort"`; a panic traps → "converter crashed").

**Job pipeline:** `commands/jobs.rs` → `queue::JobQueue` (`queue/runner.rs`) →
`engines::execute` → engine. The queue owns:
- per-job `CancellationToken`s under a root token (`cancel_all` cancels the root and then replaces it);
- three semaphore pools: `cpu` (cores/2, native engines), `sidecar` (2, CPU ffmpeg/pandoc), `gpu` (2, NVENC session caps);
- reserved output paths, so two in-flight jobs can't claim the same final name;
- a crash journal (`queue/journal.rs`) of in-flight part files, swept on the next launch.

Outputs are written to `<name>.omni-part.<ext>` and atomically renamed on success (D4,
`crates/core/src/output.rs`). "Merge to PDF" (`queue/merge.rs`) is a single job with many inputs.

**Engines are Tauri-free (D9).** Each one takes `ConvertSpec` + `JobCtx { cancel, progress: Arc<dyn ProgressSink>, tools, gpu }`.
Native engines (image/svg/pdf/data/archive) run in `spawn_blocking` and must call
`ctx.check()` periodically to cancel cooperatively. Sidecar engines (media/document) are
async. AVIF input is `MediaThenImage`: ffmpeg decodes it to a temp PNG, then the native
image encoder runs (D7). Only `TauriSink` touches `AppHandle`. It throttles progress to
about 10 events/s and ≥0.5% per job (D10).

**Sidecars (D2, D3):** spawned from Rust (`sidecar/spawn.rs`) with `tokio::process`,
`CREATE_NO_WINDOW`, and a per-run Win32 Job Object with `KILL_ON_JOB_CLOSE`. Orphans are
reaped even on a crash. The webview has **no** `shell:*` permissions, so don't add
`tauri-plugin-shell`. `sidecar/paths.rs` resolves tools next to the exe, and in debug builds
also from `src-tauri/binaries/{dev/,}`. A missing tool surfaces as `EngineError::MissingTool`.
`binaries/` and `resources/` are gitignored. Bundling config lives only in
`tauri.bundle.conf.json`, so `tauri dev` doesn't need the binaries.

**Startup:** `lib.rs` discovers tools, builds `AppState`, then probes GPU encoders in a
background task and emits `system://gpu-ready`. Window creation never waits on the probe.
`RunEvent::Exit` calls `queue.cancel_all()`.

**Events → frontend:** `conversion://progress|complete|error` are bridged once in
`src/hooks/useConversionEvents.ts`. Handlers use `store.getState()` to avoid stale closures.
Progress goes to `progressStore` (hot path, read only by ProgressBar/RowStatus), kept
separate from `queueStore` so ticks don't re-render whole rows. Keep that split.

**Tests:** engine unit tests live in `src-tauri/src/engines/tests/` and `registry/tests.rs`.
`tests/e2e_routes/` drives real files through `testkit::Harness`, the same
detect → matrix → plan → engine → finalize path the queue uses. `testkit.rs` is a narrow
`pub` re-export surface for integration tests. Don't make engine modules `pub` to reach
them, because that blinds dead-code lints. Sidecar routes are `#[ignore]`d so a clean
clone stays green.

## Tech Stack

- **Frontend:** React 19, TypeScript (strict), Tailwind v4 (`@theme` tokens in `src/styles/index.css`), `motion` (`motion/react`, not `framer-motion`), Lucide React, Zustand, `@floating-ui/react`. Web only: `@bjorn3/browser_wasi_shim`, `fflate` (Download all → ZIP).
- **Media:** ffmpeg/ffprobe sidecars. GPU (`h264_nvenc`, `hevc_nvenc`, `h264_amf`, `h264_qsv`) uses constant quality (`-cq`/`-qp`) for Fast/Balanced. CPU `libx264`/`libx265` CRF is used for Max Quality/Archival and as the fallback (`gpu/presets.rs`).
- **Image (native):** `image`, `webp` (lossy WebP), `ravif` via `image` (AVIF encode), `resvg` (SVG input only), `ico`, `lopdf` (Image→PDF, JPEG embedded as DCT without re-encode; `printpdf` dropped).
- **Data (native):** `calamine`, `rust_xlsxwriter`, `csv`, `serde_json`, `serde_yaml_ng` (not `serde_yaml`), `toml`, `quick-xml`. Markdown tables route to the data engine.
- **Documents:** pandoc sidecar with `--pdf-engine=typst`. `pdfium-render` handles PDF→image/text (`pdfium.dll` as a bundle resource).
- **Archives (native):** `zip`, `tar`, `flate2`, `sevenz-rust2`, `lzma-rust2` (pure-Rust XZ; replaced `xz2`/liblzma so core builds for WASM).

## UI/UX Rules (Obsidian Dark Minimalist)

- **Palette:** `#09090B` bg, `#121215` card, `#1E1E24` 1px borders, `#EDEDEF` text, `#8E8E93` muted, `#6366F1` indigo accent, `#10B981` success.
- **Frameless window** (`decorations: false`). Custom titlebar with drag region and min/max/close controls (`components/titlebar/`).
- **3-click workflow:** (1) full-window drop zone (Tauri native drag-drop for real paths), (2) format pill with a searchable popover showing only valid targets plus "Apply to all [ext]", (3) queue rows with input badge, target dropdown, gear (preset/GPU), live progress and "Show in Explorer".
- **Zero clutter:** advanced settings (bitrate, resolution, DPI, sample rate) stay inside the per-item gear popover (`components/settings/`, schema in `lib/settingsSchema.ts`).

## Standards

- Every hand-written TS/TSX/Rust file stays under 200 lines (enforced by `check:loc`; `src/types/generated` excluded). Split modules instead of growing them.
- Never leave orphaned `ffmpeg`/`pandoc` processes. Cancellation flows through the `CancellationToken` → Job Object. Don't bypass `sidecar::spawn::run`.
- Verify each phase with `npm run verify`. Run `npm run test:e2e` as well when touching sidecar routes.

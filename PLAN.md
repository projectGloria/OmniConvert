# OmniConvert — Implementation Plan

> Tauri v2 · React 19 · TypeScript (strict) · Tailwind v4 · Zustand · Rust/Tokio
> 100% offline. Zero network calls at runtime. Windows x64 (`x86_64-pc-windows-msvc`) only.

---

## 0. Ground Rules & Decisions (read before Phase 1)

These are the calls the rest of the plan depends on. Several of them change the original spec,
and each change says why.

| # | Decision | Why |
|---|----------|-----|
| D1 | **Rust is the single source of truth for the format matrix.** TS types are generated with `ts-rs`. The frontend never hardcodes the valid targets for an input. It calls `probe_files(paths)` and gets back `{ category, ext, targets[] }`. | A matrix kept in two languages *will* drift. A UI that offers a conversion the backend can't do is a bug factory. |
| D2 | **Sidecars are spawned from Rust via `tokio::process`, not from the webview through `tauri-plugin-shell`.** The webview gets **no** `shell:*` permissions. `bundle.externalBin` is still used for bundling. | The frontend has no reason to exec binaries. Giving the webview `shell:allow-execute` is pure attack surface. Spawning from Rust also gives us direct control over PIDs, Job Objects and `CREATE_NO_WINDOW`. *(Override if you want the plugin. It's a ~20-line change in `sidecar/spawn.rs`.)* |
| D3 | **Every sidecar child is assigned to a Win32 Job Object with `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`.** | Tracking PIDs in state only covers *graceful* exits. If the app crashes or gets killed from Task Manager, `Drop` never runs, and you get orphaned `ffmpeg.exe` processes eating the GPU. The Job Object handle closes when the process dies, so the OS reaps the children. PIDs are still stored for per-job Cancel. |
| D4 | **All outputs are written to `<name>.<ext>.part` and atomically renamed on success.** | Cancel or failure never leaves half-written files that look valid in Explorer. |
| D5 | **`serde_yaml` → `serde_yaml_ng`.** | `serde_yaml` was archived and marked unmaintained by its author in 2024. Shipping a dead crate in a new project is a bad start. Same serde API, drop-in. |
| D6 | **`printpdf` dropped in favour of `lopdf` alone for Image→PDF.** | `printpdf` 0.8 rewrote its whole API. More to the point, `lopdf` can embed JPEG bytes **as-is** (`DCTDecode`) with no re-encoding: lossless and much faster. PNG goes through `FlateDecode` with an `SMask` for alpha. |
| D7 | **Lossy WebP uses the `webp` crate. AVIF *encode* uses `image`'s `avif` feature (which is `ravif`). AVIF *decode* goes through the ffmpeg sidecar.** | `image` only encodes lossless WebP. `image`'s native AVIF decode needs `dav1d` as a system C library, which is painful on MSVC. ffmpeg already ships `libdav1d`, so we pipe it: `ffmpeg -i in.avif -f image2pipe -c:v png -`. |
| D8 | **`framer-motion` → `motion` package** (`import { motion } from "motion/react"`). | Same library under its new name. `framer-motion` is the legacy alias. |
| D9 | **Engines don't depend on Tauri.** Each converter takes `&JobCtx { cancel: CancellationToken, progress: &dyn ProgressSink }`. | This makes the converters testable with `cargo test` without starting a Tauri app (Phase 5). The Tauri `AppHandle` only appears in the `ProgressSink` implementation. |
| D10 | **Event throttling:** at most ~10 progress events per second per job (emit only when progress moves ≥0.5% **and** at least 100 ms have passed). | ffmpeg `-progress` emits continuously. Unthrottled IPC with 8 parallel jobs makes React re-render the queue hundreds of times a second. |

### Honest scope cuts (things that are physically not going to happen offline/cleanly)

- **PDF → DOCX / MD / HTML is not offered.** Pandoc **cannot read PDF**. Rebuilding a layout from a PDF is an OCR and layout-analysis project of its own. What PDF *can* do as input: **PDF → PNG/JPG** (pdfium, one file per page, zipped if there are several pages) and **PDF → TXT** (pdfium text extraction).
- **Raster → SVG is not offered.** That would be vector tracing (potrace), not conversion. SVG is **input-only** and gets rasterised by `resvg`.
- **DOCX → PDF via pandoc + typst is a re-typeset, not a faithful render.** Fonts, headers, floating images and Word-specific layout will change. The UI shows a one-line "layout may differ" hint for this route.
- **ODS / XLS are read-only** (`calamine`). No maintained pure-Rust writer exists for either. Spreadsheet output is XLSX.
- **GZ / XZ are single-stream compressors.** For a multi-entry archive the target becomes `TAR.GZ` / `TAR.XZ`. A plain `.gz`/`.xz` target only shows up when the source has exactly one entry.
- **Licensing:** ffmpeg (GPL builds with x264/x265) and pandoc (GPL-2.0+) are bundled. Distributing the installer brings GPL source-offer obligations for those binaries. That's fine for personal use; deal with it before any public release.

---

## 1. Folder Structure

```
OmniConvert/
├─ CLAUDE.md
├─ PLAN.md
├─ package.json
├─ tsconfig.json
├─ vite.config.ts
├─ index.html
├─ scripts/
│  ├─ fetch-sidecars.ps1          # dev-time only: download + verify + rename binaries
│  └─ sidecars.lock.json          # pinned versions + SHA256 per binary
├─ src/                           # React frontend (every file < 200 lines)
│  ├─ main.tsx
│  ├─ App.tsx
│  ├─ styles/
│  │  └─ index.css                # Tailwind v4 @import + @theme tokens (Obsidian palette)
│  ├─ types/
│  │  └─ generated/               # ts-rs output from Rust — DO NOT EDIT
│  ├─ lib/
│  │  ├─ ipc.ts                   # typed invoke() wrappers
│  │  ├─ events.ts                # listen() for conversion://* with cleanup
│  │  ├─ format.ts                # bytes/duration formatters
│  │  └─ cn.ts                    # className joiner
│  ├─ store/
│  │  ├─ queueStore.ts            # items, targets, settings (Zustand)
│  │  ├─ progressStore.ts         # hot path: per-job progress, separate from queue
│  │  └─ appStore.ts              # outputDir, gpu caps, concurrency
│  ├─ hooks/
│  │  ├─ useFileDrop.ts           # getCurrentWebview().onDragDropEvent
│  │  ├─ useConversionEvents.ts   # wires backend events → stores
│  │  └─ useHotkeys.ts
│  └─ components/
│     ├─ titlebar/  Titlebar.tsx, WindowControls.tsx
│     ├─ dropzone/  DropOverlay.tsx, EmptyState.tsx
│     ├─ queue/     QueueList.tsx, QueueRow.tsx, ProgressBar.tsx, InputBadge.tsx, RowActions.tsx
│     ├─ picker/    FormatPill.tsx, FormatPopover.tsx, FormatSearch.tsx, ApplyAllButton.tsx
│     ├─ settings/  SettingsPopover.tsx, PresetSegment.tsx, VideoFields.tsx, AudioFields.tsx, ImageFields.tsx
│     ├─ bottombar/ BottomBar.tsx, OutputFolderPicker.tsx, ConvertAllButton.tsx
│     └─ summary/   BatchSummary.tsx
└─ src-tauri/
   ├─ Cargo.toml
   ├─ build.rs
   ├─ tauri.conf.json
   ├─ capabilities/
   │  └─ default.json
   ├─ binaries/                   # gitignored; filled by fetch-sidecars.ps1
   │  ├─ ffmpeg-x86_64-pc-windows-msvc.exe
   │  ├─ ffprobe-x86_64-pc-windows-msvc.exe
   │  ├─ pandoc-x86_64-pc-windows-msvc.exe
   │  └─ typst-x86_64-pc-windows-msvc.exe
   ├─ resources/
   │  └─ pdfium.dll               # gitignored; bundle.resources (NOT externalBin — it's a DLL)
   ├─ tests/
   │  ├─ fixtures.rs              # generates sample JSON/CSV/PNG/WAV/MD at test time
   │  └─ e2e_routes.rs            # route-by-route conversion tests
   └─ src/
      ├─ main.rs                  # just calls lib::run()
      ├─ lib.rs                   # builder, plugins, state, command registration
      ├─ error.rs                 # AppError (thiserror) → serialisable to TS
      ├─ state.rs                 # AppState { queue, gpu_caps, paths }
      ├─ commands/
      │  ├─ mod.rs
      │  ├─ probe.rs              # probe_files
      │  ├─ jobs.rs               # enqueue, cancel_job, cancel_all
      │  └─ system.rs             # gpu_caps, reveal, open_dir, default_output_dir
      ├─ registry/
      │  ├─ mod.rs
      │  ├─ formats.rs            # Format enum + ext/mime/category metadata
      │  ├─ matrix.rs             # ConversionMatrix: (Format) -> &[Target]
      │  ├─ detect.rs             # extension + magic bytes (infer) → Format
      │  └─ route.rs              # (src, dst) -> EngineRoute
      ├─ queue/
      │  ├─ mod.rs
      │  ├─ job.rs                # Job, JobId, JobSpec, JobStatus
      │  ├─ runner.rs             # dispatch loop, per-pool semaphores
      │  ├─ cancel.rs             # CancellationToken registry
      │  ├─ progress.rs           # ProgressSink trait + throttled Tauri impl
      │  └─ output.rs             # .part temp path, collision naming "x (1).png", atomic rename
      ├─ sidecar/
      │  ├─ mod.rs
      │  ├─ paths.rs              # resolve sidecar exe next to app exe; dunce paths
      │  ├─ spawn.rs              # tokio Command + CREATE_NO_WINDOW + job object assign
      │  ├─ job_object.rs         # Win32 Job Object (KILL_ON_JOB_CLOSE)
      │  └─ registry.rs           # JobId → child PID map, kill on cancel/exit
      ├─ gpu/
      │  ├─ mod.rs
      │  ├─ detect.rs             # real 1-frame encode tests per encoder
      │  └─ presets.rs            # Preset × Encoder → ffmpeg args
      └─ engines/
         ├─ mod.rs                # trait Converter + JobCtx
         ├─ media/   mod.rs, probe.rs (ffprobe JSON), transcode.rs, audio.rs, animated.rs, frames.rs, progress_parse.rs
         ├─ image/   mod.rs, decode.rs, encode.rs, svg.rs, ico.rs, to_pdf.rs
         ├─ pdf/     mod.rs, render.rs (pdfium → PNG/JPG), text.rs
         ├─ data/    mod.rs, model.rs (Table + Tree IR), read.rs, write.rs, spreadsheet.rs, markdown.rs, xml.rs
         ├─ document/ mod.rs, pandoc.rs
         └─ archive/ mod.rs, model.rs (entry stream IR), read.rs, write.rs, sevenz.rs
```

---

## 2. Rust Dependencies — `src-tauri/Cargo.toml`

> These versions are **floors** that I'm confident exist. In Phase 1 every one gets resolved with
> `cargo add` against the current crates.io index, and `Cargo.lock` gets committed. Several of these
> crates (zip, quick-xml, calamine, sevenz-rust2) ship breaking minors often, so we don't guess.

```toml
[package]
name = "omniconvert"
version = "0.1.0"
edition = "2021"
rust-version = "1.80"

[lib]
name = "omniconvert_lib"
crate-type = ["staticlib", "cdylib", "rlib"]

[build-dependencies]
tauri-build = { version = "2", features = [] }

[dependencies]
# --- Tauri core + plugins (no tauri-plugin-shell: see D2) ---
tauri               = { version = "2", features = [] }
tauri-plugin-dialog = "2"          # output folder picker
tauri-plugin-opener = "2"          # revealItemInDir / openPath (Explorer integration)

# --- Async runtime / queue ---
tokio      = { version = "1", features = ["rt-multi-thread", "macros", "process", "sync", "io-util", "time", "fs"] }
tokio-util = { version = "0.7", features = ["rt"] }   # CancellationToken
uuid       = { version = "1", features = ["v4", "serde"] }

# --- Errors, serde, TS codegen ---
thiserror  = "2"
serde      = { version = "1", features = ["derive"] }
serde_json = "1"
ts-rs      = { version = "10", features = ["serde-compat", "uuid-impl"] }

# --- Detection / paths ---
infer   = "0.16"       # magic-byte sniffing (don't trust extensions blindly)
dunce   = "1"          # strip \\?\ verbatim prefixes before handing paths to ffmpeg/pandoc
walkdir = "2"

# --- Image engine ---
image  = { version = "0.25", default-features = false, features = ["png", "jpeg", "gif", "bmp", "tiff", "ico", "webp", "avif", "rayon"] }  # "avif" = ravif encoder
webp   = "0.3"         # lossy WebP encode (image crate is lossless-only)
resvg  = "0.45"        # SVG raster (re-exports usvg + tiny-skia)
ico    = "0.3"         # multi-resolution .ico (16/32/48/64/128/256)

# --- PDF ---
lopdf        = "0.34"  # Image→PDF assembly (JPEG passthrough via DCTDecode)
pdfium-render = "0.8"  # PDF→image/text; binds pdfium.dll from resource dir at runtime

# --- Data / spreadsheet engine ---
calamine        = "0.26"
rust_xlsxwriter = "0.80"
csv             = "1"
serde_yaml_ng   = "0.10"   # see D5
toml            = "0.8"
quick-xml       = { version = "0.37", features = ["serialize"] }
pulldown-cmark  = { version = "0.12", default-features = false }   # Markdown table parsing (tables ext)

# --- Archive engine ---
zip          = { version = "2", default-features = false, features = ["deflate", "bzip2", "zstd", "time"] }
tar          = "0.4"
flate2       = "1"
xz2          = { version = "0.1", features = ["static"] }   # liblzma statically linked; `liblzma` crate is the maintained drop-in if xz2 bites
sevenz-rust2 = { version = "0.13", features = ["compress"] }

# --- Logging (file log in %LOCALAPPDATA%, never network) ---
tracing            = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "fmt"] }

[target.'cfg(windows)'.dependencies]
windows-sys = { version = "0.59", features = [
  "Win32_Foundation",
  "Win32_Security",
  "Win32_System_JobObjects",
  "Win32_System_Threading",
] }

[dev-dependencies]
tempfile = "3"
hound    = "3"   # WAV fixture generation

[profile.dev.package."*"]
opt-level = 2    # image/rav1e/resvg at opt-level 0 make dev-mode conversions unusably slow

[profile.release]
opt-level     = 3
lto           = "fat"
codegen-units = 1
strip         = true
panic         = "unwind"   # keep unwind: a panicking converter must fail its job, not the app
```

> ⚠ `ravif` pulls in `rav1e`. With its `asm` feature on, the build needs **NASM** on PATH. Phase 1 checks
> for `nasm -v`. If it's missing, we either install it or turn `asm` off with a feature override (slower
> AVIF encoding, but no extra toolchain).

### Frontend dependencies — `package.json`

| Runtime | Dev |
|---|---|
| `react@^19`, `react-dom@^19` | `typescript@^5`, `vite`, `@vitejs/plugin-react` |
| `@tauri-apps/api@^2` | `@tauri-apps/cli@^2` |
| `@tauri-apps/plugin-dialog@^2`, `@tauri-apps/plugin-opener@^2` | `tailwindcss@^4`, `@tailwindcss/vite` |
| `zustand@^5`, `motion`, `lucide-react` | `@types/react`, `@types/react-dom` |
| `@floating-ui/react` (popover positioning + focus management) | |

Scripts: `dev`, `build`, `tauri`, `typecheck` (`tsc --noEmit`), `bindings` (`cargo test --manifest-path src-tauri/Cargo.toml export_bindings` to regenerate `ts-rs` types), `test:e2e`.

---

## Phase 1 — Tauri v2 Scaffolding & Frameless Obsidian UI

**Goal:** an app that launches with no window decorations, a working custom titlebar, live drag-drop of real file paths into a static queue, and the bottom bar. No conversions yet.

### 1.1 Scaffold
- `npm create tauri-app@latest` → React + TS + Vite template, run in place. Prune the template junk.
- Tailwind v4 via `@tailwindcss/vite`. Palette tokens go in `@theme` in `src/styles/index.css`:
  `--color-bg: #09090B; --color-surface: #121215; --color-border: #1E1E24; --color-text: #EDEDEF; --color-muted: #8E8E93; --color-accent: #6366F1; --color-success: #10B981;`
- `tsconfig.json`: `strict`, `noUncheckedIndexedAccess`, `exactOptionalPropertyTypes`, `verbatimModuleSyntax`.
- Add `.gitignore` entries for `src-tauri/binaries/*.exe`, `src-tauri/resources/*.dll`, `target/`, `dist/`.

### 1.2 `tauri.conf.json` (key parts)
```jsonc
{
  "identifier": "com.omniconvert.app",
  "app": {
    "windows": [{
      "label": "main",
      "title": "OmniConvert",
      "width": 1040, "height": 720, "minWidth": 760, "minHeight": 520,
      "decorations": false,
      "shadow": true,            // keeps the Win11 drop shadow + rounded corners on an undecorated window
      "dragDropEnabled": true,   // native drop → real absolute paths (no browser File objects)
      "center": true
    }],
    "security": { "csp": "default-src 'self'; img-src 'self' asset: data:; style-src 'self' 'unsafe-inline'; connect-src ipc: http://ipc.localhost" }
  },
  "bundle": {
    "externalBin": ["binaries/ffmpeg", "binaries/ffprobe", "binaries/pandoc", "binaries/typst"],
    "resources": { "resources/pdfium.dll": "pdfium.dll" }
  }
}
```
The CSP has no remote origins at all. That's the offline guarantee enforced at the webview level, not just by convention.

### 1.3 `capabilities/default.json`
```jsonc
{
  "identifier": "default",
  "windows": ["main"],
  "permissions": [
    "core:default",
    "core:window:allow-start-dragging",
    "core:window:allow-minimize",
    "core:window:allow-toggle-maximize",
    "core:window:allow-is-maximized",
    "core:window:allow-close",
    "core:event:default",
    "core:webview:default",
    "dialog:allow-open",
    "opener:allow-reveal-item-in-dir",
    "opener:allow-open-path"
  ]
}
```
There are no `shell:*` permissions on purpose (D2). Custom `#[tauri::command]`s are allowed by default for app-defined commands.

### 1.4 UI components
- **Titlebar:** 36px, `data-tauri-drag-region` on the root and the title span (drag regions don't inherit into children). It has min / max-restore / close buttons, and the close button turns red on hover. A double-click on the drag region toggles maximise natively. Known trade-off: you lose Win11 Snap Layouts on hover over maximise. Bringing them back means `WM_NCHITTEST` hacking, which is out of scope.
- **DropOverlay:** a full-window overlay animated with `motion` (`AnimatePresence`), driven by `onDragDropEvent` `enter/over/leave/drop`. The `drop` event hands its paths to `probe_files` (stubbed in Phase 1 to return the extension only).
- **QueueRow (static):** input badge, filename, size, a target pill placeholder, a gear placeholder, and a progress track.
- **BottomBar:** sticky. Holds the output folder picker (`dialog.open({ directory: true })`, default "Same as source") and the **Convert All** CTA, which is disabled when no row has a target. That disabled state is *derived* from the store, not kept in its own `useState`.

### 1.5 State architecture (set up now, because it's painful to retrofit)
- `queueStore`: `Record<JobId, QueueItem>` + `order: JobId[]`. Components subscribe through narrow selectors (`useQueue(s => s.items[id].target)`).
- `progressStore` is **separate**. Progress ticks only touch this store, and only the matching `<ProgressBar id>` subscribes. The queue list itself never re-renders on a tick.
- No `useEffect` that syncs one piece of state into another. Values like "all done", "any running" and counts per category are computed from the store with selectors.

**Exit check:** `npm run typecheck` ✓ · `cargo check` ✓ · `npm run tauri dev` → drag files in, rows appear with correct paths, window controls work.

---

## Phase 2 — Universal Format Registry & Compatibility Matrix

**Goal:** Rust knows every format and every legal route. The frontend gets valid targets per file.

### 2.1 Schema (Rust, exported to TS via `ts-rs`)
```rust
enum Category { Video, Audio, Image, Document, Data, Archive }
enum Format { Mp4, Mkv, Mov, Avi, Webm, Flv, Mp3, Wav, Flac, Aac, Ogg, M4a, Opus,
              Png, Jpg, Webp, Avif, Ico, Bmp, Tiff, Gif, Svg, Pdf,
              Docx, Md, Html, Txt, Rtf, Epub,
              Xlsx, Xls, Ods, Csv, Tsv, Json, Yaml, Toml, Xml, MdTable,
              Zip, SevenZ, Tar, Gz, Tgz, Xz, Txz }
enum TargetKind { Convert(Format), AnimatedGif, AnimatedWebp, FramesZip, PdfPagesPng, PdfPagesJpg, MergeToPdf, Extract }
struct Target { kind: TargetKind, label: String, group: Category, note: Option<String> }  // note = "layout may differ"
struct FileInfo { path: String, format: Format, category: Category, size: u64, targets: Vec<Target>, meta: Option<MediaMeta> }
```
`EngineRoute` = `Media | Image | Pdf | Data | Document | Archive | MediaThenImage` (for AVIF decode, D7).

### 2.2 Detection
Detection is extension first, then `infer` magic bytes to confirm or override. A mismatch (e.g. a `.png` that's really a JPEG) is trusted to the magic bytes, and the row shows a subtle "detected as JPG" hint. Ambiguous containers like `.ogg` (audio vs. Theora video) and `.webm` (could be audio-only) get **ffprobe stream info** in `probe_files`. The category comes from the actual streams.

### 2.3 Conversion matrix

| Source | Targets |
|---|---|
| **Video** MP4 MKV MOV AVI WEBM FLV | other video containers · MP3 WAV FLAC AAC OGG M4A OPUS · Animated GIF · Animated WebP · Frames→ZIP(PNG) |
| **Audio** MP3 WAV FLAC AAC OGG M4A OPUS | any other audio |
| **Animated GIF** | MP4 · WEBM · Animated WebP · (static) PNG/JPG first frame |
| **Image** PNG JPG WEBP AVIF BMP TIFF ICO GIF | each other (no SVG out) · PDF |
| **SVG** | PNG JPG WEBP AVIF ICO BMP TIFF · PDF |
| **Multiple images (batch action)** | **Merge → single PDF** (in queue order) |
| **PDF** | PNG pages · JPG pages · TXT |
| **Document** DOCX MD HTML TXT RTF EPUB | DOCX MD HTML TXT RTF EPUB PDF (pandoc; PDF via `--pdf-engine=<abs path to typst sidecar>`) |
| **Spreadsheet** XLSX XLS ODS | CSV TSV JSON YAML TOML XML MD-Table · XLSX (from XLS/ODS) |
| **Tabular text** CSV TSV MD-Table | each other · XLSX · JSON YAML TOML XML |
| **Tree data** JSON YAML TOML XML | each other · CSV/TSV/XLSX/MD-Table **only when the root is an array of flat objects** (checked at probe time, so the target simply doesn't appear otherwise) |
| **Archive** ZIP 7Z TAR TGZ TXZ | each other · Extract to folder |
| **GZ / XZ** (single stream) | decompress · repack to ZIP/7Z |

### 2.4 Data-engine IR (why this isn't N² converters)
Data uses two intermediate shapes: **`Table`** (`headers: Vec<String>`, `rows: Vec<Vec<Cell>>`) and **`Tree`** (`serde_json::Value`). Every reader produces one of them and every writer consumes one, so 10 formats need about 20 adapters instead of 90 point-to-point converters. Two edge cases to handle:
- TOML has no top-level arrays and no null. Tables are written as `[[rows]]`, and nulls are left out.
- XML has no canonical tabular form. The convention is `<rows><row><col>…</col></row></rows>` on write, and "repeated sibling elements = rows" on read.

Archives work the same way: a streaming **entry iterator** IR (`path, mode, mtime, Read`), so repacking never extracts to disk first.

**Exit check:** a unit test that every `Format` has ≥1 target, that no target maps back to the source format, and that every `TargetKind` resolves to an `EngineRoute`. TS bindings regenerate cleanly. The frontend shows real target lists.

---

## Phase 3 — Job Queue, Pure-Rust Engines & Sidecar Pipeline

### 3.1 `JobQueue`
- `enqueue(Vec<JobSpec>) -> Vec<JobId>`, `cancel_job(id)`, `cancel_all()`.
- **Three pools, each with its own `Semaphore`:**
  - `cpu`: pure-Rust image/data/archive work. Runs in `spawn_blocking`. Permits = `available_parallelism() / 2`, minimum 1.
  - `sidecar`: CPU ffmpeg and pandoc. Permits = 2. x264/x265 already use every core, so running more in parallel only adds contention.
  - `gpu`: hardware encodes. Permits = 2. NVENC has a per-GPU session cap on consumer cards, and staying under it avoids `OpenEncodeSessionEx failed`.
- Each job gets a child `CancellationToken` of a root token, so `cancel_all` is a single call.
- Events (payloads typed through `ts-rs`):
  - `conversion://progress` `{ jobId, progress: f32 | null, stage }`. `null` = indeterminate (pandoc, archive with unknown total).
  - `conversion://complete` `{ jobId, outputs: Vec<PathBuf>, elapsedMs }`
  - `conversion://error` `{ jobId, code, message, cancelled: bool }`
- A panic inside a converter is caught by `JoinHandle` → JoinError, turned into `conversion://error`, and the queue keeps going.

### 3.2 Pure-Rust engines (built first, because they need no external binaries)
- **Image:** decode with `image` (SVG via `resvg` at a target DPI/size, AVIF via ffmpeg pipe), then optional resize (`imageops::resize`, Lanczos3), then encode. JPG gets a quality slider and alpha flattened onto a configurable background (default white). WebP lossy uses the `webp` crate. ICO is multi-size, using the sizes ≤ the source. Merge→PDF uses `lopdf`: one page per image, page size = image size at 72dpi or a fit-to-A4 option. JPEG bytes are passed through untouched.
- **Data:** readers/writers as in 2.4. XLSX output gets a bold header row, a frozen first row and auto-width columns. Multi-sheet XLSX input → a sheet picker in settings (default: first sheet; for JSON/YAML/TOML targets, "all sheets" makes `{ sheetName: rows[] }`).
- **Archive:** streaming entry IR. Cancel is checked between entries and every 4 MiB of copy. Zip-slip protection on extract: canonicalise each entry path and reject anything outside the destination.
- **PDF:** `pdfium-render`, bound at runtime from `app.path().resource_dir()/pdfium.dll`. Pages render at a configurable DPI (default 150). Output is a folder or ZIP when there's more than one page.

### 3.3 Sidecar layer
- `paths.rs`: Tauri strips the target-triple suffix and places sidecars next to the main exe, so we resolve them with `std::env::current_exe()?.parent()?.join("ffmpeg.exe")`. In `tauri dev` they live in `target/debug/`, also next to the exe. Every argument path goes through `dunce::simplified`.
- `spawn.rs`: `tokio::process::Command` with `creation_flags(CREATE_NO_WINDOW)` (otherwise a console flashes for every job), `kill_on_drop(true)`, stdout/stderr piped. Right after spawning, the child is assigned to the app-wide Job Object (D3).
- `registry.rs`: `Mutex<HashMap<JobId, u32 /*pid*/>>`. Cancel = `child.start_kill()`. On window `CloseRequested` / `RunEvent::Exit`: cancel the root token, kill everything in the registry, then let the Job Object cover anything we missed.
- **ffmpeg progress:** `ffprobe -v error -show_entries format=duration -of json` → total µs. Then run `ffmpeg -hide_banner -nostats -progress pipe:1 …` and parse `out_time_us=` lines. stderr is captured into a 64-line ring buffer that gets included in error messages.
- **Pandoc:** `pandoc in -o out.part.ext [--pdf-engine=<typst abs path>]`. Progress is indeterminate. The working directory is the source's directory so relative images resolve. `--resource-path` is set as well.

### 3.4 `scripts/fetch-sidecars.ps1` (dev machine only. The installed app never downloads anything.)
- Reads `scripts/sidecars.lock.json`: pinned version, URL and **SHA256** for each artifact. A hash mismatch is a hard fail. We don't bundle binaries we haven't verified.
- Sources (official release channels):
  - ffmpeg + ffprobe: gyan.dev `ffmpeg-release-essentials.zip`, or BtbN `ffmpeg-n*-win64-gpl.zip` on GitHub releases. Whichever we pick must report `libx264`, `libx265`, `h264_nvenc`, `hevc_nvenc`, `h264_amf`, `h264_qsv`, `libwebp`, `libdav1d` in `ffmpeg -encoders`/`-decoders`. The script asserts this after extraction.
  - pandoc: `jgm/pandoc` releases, `pandoc-<v>-windows-x86_64.zip`
  - typst: `typst/typst` releases, `typst-x86_64-pc-windows-msvc.zip`
  - pdfium: `bblanchon/pdfium-binaries` releases, `pdfium-win-x64.tgz` → `bin/pdfium.dll`
- Downloads to a fresh temp dir and extracts there, then copies the files into `src-tauri/binaries/` as `<name>-x86_64-pc-windows-msvc.exe` (and `pdfium.dll` into `resources/`). It's idempotent: if the hash already matches, the download is skipped. Flags: `-Force`, `-Only ffmpeg,pandoc`.
- Run with `pwsh -ExecutionPolicy Bypass -File scripts/fetch-sidecars.ps1`.

### 3.5 GPU detection (`gpu/detect.rs`)
- At startup, in the background (it doesn't block the window), run real 1-frame encodes in parallel:
  `ffmpeg -hide_banner -v error -f lavfi -i color=c=black:s=256x256:d=0.04 -frames:v 1 -c:v <enc> -f null -`
  The exit code decides availability. **Parsing `-encoders` is not enough.** `h264_nvenc` is listed in every NVENC-enabled build even on a machine with no NVIDIA GPU.
- Encoders tested: `h264_nvenc`, `hevc_nvenc`, `h264_amf`, `hevc_amf`, `h264_qsv`, `hevc_qsv`. The result is cached in `AppState.gpu_caps` and exposed through `gpu_caps()`. It's also emitted as `system://gpu-ready` so the settings popover can enable/disable presets.
- **Presets (`gpu/presets.rs`):**

| Preset | Encoder priority | Quality args |
|---|---|---|
| Fast | nvenc → amf → qsv → libx264 | NVENC `-preset p2 -rc vbr -cq 28 -b:v 0` · AMF `-quality speed -rc cqp -qp_i 26 -qp_p 28` · QSV `-preset veryfast -global_quality 28` · CPU `-preset veryfast -crf 26` |
| Balanced | nvenc → amf → qsv → libx264 | NVENC `-preset p5 -rc vbr -cq 23 -b:v 0` · AMF `-quality balanced -rc cqp -qp_i 22 -qp_p 24` · QSV `-preset medium -global_quality 23` · CPU `-preset medium -crf 22` |
| Archival | **always CPU** libx265 | `-preset slow -crf 20 -tag:v hvc1` (for MP4/MOV, so Apple players accept it) |

The container decides the codec. WEBM forces `libvpx-vp9`/`libopus` (CPU, `-crf 32 -b:v 0 -row-mt 1`), and FLV/AVI fall back to H.264. If a GPU encode fails mid-job (driver hiccup, session cap), the job retries once on CPU instead of erroring.

- **Animated GIF:** two-pass `fps=15,scale=<w>:-1:flags=lanczos,split[a][b];[a]palettegen=stats_mode=diff[p];[b][p]paletteuse=dither=bayer:bayer_scale=5`.
- **Animated WebP:** `-c:v libwebp_anim -loop 0 -q:v 75`.
- **Frames→ZIP:** extract to a temp dir with `-vf fps=<n>`, then zip with `zip` (stored, since PNG is already compressed), then delete the temp dir.

**Exit check:** `cargo check` ✓, `cargo test` for the pure-Rust engines ✓, a manual run of MP4→MP3 and MP4→MP4 (Balanced) with live progress and Cancel mid-encode, and Task Manager showing no leftover `ffmpeg.exe` after Cancel **and** after killing the app process.

### 3.6 As built (differences from the plan above)
- **One Job Object per sidecar run instead of one for the whole app.** Pandoc spawns `typst` as a grandchild, so killing pandoc's PID would leave typst orphaned. `TerminateJobObject` kills the whole tree on Cancel. Closing the handle (when the job ends or the app crashes) does the same through `KILL_ON_JOB_CLOSE`. The per-job `CancellationToken` map is the "active PIDs in state" registry.
- **`externalBin` and `resources` moved to `src-tauri/tauri.bundle.conf.json`, used by `npm run tauri:build`.** `tauri-build` checks that those files exist even for `cargo check`, so keeping them in the base config would break every build until the sidecars are fetched. In dev, the app finds the tools via `src-tauri/binaries/dev/<name>.exe`, which are hard links created by the fetch script. They need plain names because pandoc recognises `--pdf-engine` by its basename.
- **NASM is not needed.** `image` pulls in `ravif` with default features off, so rav1e's assembly isn't built. AVIF encoding is slower, but there's no extra toolchain.
- **quick-xml is pinned at 0.37.** From 0.38 on, entities arrive as separate `GeneralRef` events, which is more machinery than a small XML↔data adapter needs.
- **serde_json uses `preserve_order`.** Without it, JSON objects come out with alphabetical keys, so CSV→JSON would scramble the column order.
- **Part files are named `<name>.omni-part.<ext>`.** Keeping the real extension lets ffmpeg and pandoc infer the output container, and the infix still makes partial files obvious. They're deleted on failure or cancel.
- **The `ico` crate was dropped.** `image`'s `IcoEncoder::encode_images` already writes multi-size icons.
- **The fetch script runs on Windows PowerShell 5.1** (`pwsh` isn't installed). It is trust-on-first-use: `-UpdateLock` resolves the releases and records their SHA-256 hashes in `scripts/sidecars.lock.json`, and later runs check against those hashes.

---

## Phase 4 — UX Polish: Format Picker, Settings Drawer, Explorer Integration

- **FormatPopover:** `@floating-ui/react` handles positioning, focus trap, Esc and click-outside. Targets are grouped by `Category` headers and the search input is auto-focused. Arrow keys move through a flat index across groups, Enter selects, typing filters by label and extension. Each target shows its `note`, e.g. "layout may differ".
- **Apply to all `.ext`:** a footer action in the popover that sets the same target on every queued item of the same `Format` that doesn't have a target yet (with Shift: all of them, overwriting). Separately, multi-image selection → **Merge to PDF** creates one group job.
- **SettingsPopover (gear, per row, only rendered when the item has settings):**
  - Video: preset segment (Fast · Balanced · Archival). GPU presets show a small chip with the detected encoder ("NVENC"), or are disabled with a tooltip when no GPU encoder was found. Scale: Original / 1080p / 720p / 480p / custom width. FPS cap for GIF/WebP.
  - Audio: bitrate (96–320k) for lossy targets, sample rate (Original/44.1/48k).
  - Image: quality 1–100 (JPG/WebP/AVIF), resize long edge, background colour for alpha flatten, DPI for SVG/PDF render.
  - Settings are kept as a sparse diff per item, and defaults are derived from target+preset. Changing the target then can't leave stale fields behind.
- **Row lifecycle:** queued → running (animated bar, `motion` layout) → done (emerald check plus **Reveal** = `revealItemInDir(output)`) | error (message, expandable stderr tail, Retry) | cancelled.
- **BatchSummary:** when every job in the batch reaches a final state: "12 converted · 1 failed · 2.4 GB → 610 MB · 1m 42s", plus **Open Output Folder** (`openPath(dir)`). It's derived from the store, not a separate effect-driven state.
- **Hotkeys:** `Ctrl+O` add files, `Ctrl+Enter` convert all, `Esc` close popovers, `Del` remove selected row(s).

**Exit check:** typecheck ✓, `cargo check` ✓, the full 3-click flow works by keyboard only.

---

## Phase 5 — Verification & Self-Contained Windows Build

### 5.1 Static checks
- `npm run typecheck` and `cargo check`, plus `cargo clippy -- -D warnings`.
- A line-count guard: `scripts/check-loc.ps1` fails if any `src/**/*.{ts,tsx}` or `src-tauri/src/**/*.rs` file is over 200 lines.

### 5.2 End-to-end route tests (`src-tauri/tests/`)
- `fixtures.rs` generates its inputs at test time into a `tempfile::TempDir`: JSON (array of objects), CSV, a 64×64 PNG with alpha (`image`), a 1-second 440Hz WAV (`hound`), and an MD file with a table and headings. No binary fixtures are committed.
- `e2e_routes.rs` builds each conversion straight from the engines (D9), with no Tauri runtime, and asserts that the output exists, is non-empty, has the right magic bytes, and **round-trips where that's meaningful** (CSV→JSON→CSV gives equal rows, PNG→WEBP(lossless)→PNG gives identical pixels).
- Sidecar routes (WAV→MP3/FLAC/OPUS, MD→DOCX/HTML/PDF) are `#[ignore]` by default and run with `--ignored` only when `src-tauri/binaries/` is populated. CI-less local dev shouldn't fail just because the binaries weren't fetched.
- `npm run test:e2e` = `cargo test --manifest-path src-tauri/Cargo.toml --test e2e_routes -- --include-ignored`.

### 5.3 Installer (`tauri.conf.json` → `bundle`)
```jsonc
"bundle": {
  "active": true,
  "targets": ["nsis", "msi"],
  "icon": ["icons/32x32.png", "icons/128x128.png", "icons/icon.ico"],
  "externalBin": ["binaries/ffmpeg", "binaries/ffprobe", "binaries/pandoc", "binaries/typst"],
  "resources": { "resources/pdfium.dll": "pdfium.dll" },
  "windows": {
    "webviewInstallMode": { "type": "offlineInstaller" },  // embeds WebView2 (~130MB) → installs with no network
    "nsis": { "installMode": "perMachine", "compression": "lzma" },
    "wix":  { "language": "en-US" }
  }
}
```
- `offlineInstaller` vs. `skip`: Win11 ships WebView2, but "100% offline installer" has to mean it, so it's embedded. That's +~130 MB. Switch to `skip` if you only target stock Win11.
- **Expected installer size: roughly 350–450 MB**, mostly ffmpeg (~80–130 MB depending on the build), pandoc (~150 MB+) and WebView2. That's the cost of "universal + offline", and Phase 5 measures it rather than guessing.
- Final smoke test: install the NSIS build on a machine/VM with the **network adapter disabled**, then convert one file per domain.

---

## Verification Gate (every phase)
```
npm run typecheck
cd src-tauri && cargo check
```
Phases don't move forward with red checks.

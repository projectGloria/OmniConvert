# Project: OmniConvert (Tauri v2 Offline Universal File Converter for Windows)

## 1. Tech Stack & Core Architecture
- **Desktop Framework:** Tauri v2 (`x86_64-pc-windows-msvc` target), 100% offline execution, zero network calls.
- **Frontend:** React 19, TypeScript (strict), Tailwind CSS, Framer Motion, Lucide React, Zustand.
- **Backend (Rust `src-tauri`):** Asynchronous Tokio job queue executing native Rust converters and embedded Tauri sidecars (`bundle.externalBin`).
- **Offline Conversion Engines:**
  1. **Media Engine (Sidecar):** Bundled `ffmpeg-x86_64-pc-windows-msvc.exe` and `ffprobe-x86_64-pc-windows-msvc.exe` for all Video <-> Video, Video -> Audio, Audio <-> Audio, and Video -> Animated GIF/WebP conversions.
  2. **Hardware Acceleration:** Probe GPU capabilities on app startup (`h264_nvenc`, `hevc_nvenc`, `h264_amf`, `h264_qsv`). Use GPU with Constant Quality (`-cq`/`-qp`) for "Fast" and "Balanced" presets, and CPU (`libx264`/`libx265` CRF) for "Max Quality/Archival" or fallback.
  3. **Image Engine (Native Rust):** Use `image`, `webp`, `ravif`, `resvg` (SVG), `ico`, and `lopdf`/`printpdf` crates directly in Rust threads for PNG, JPG, WEBP, AVIF, BMP, TIFF, ICO, SVG, GIF, and Image-to-PDF.
  4. **Data & Spreadsheet Engine (Native Rust):** Use `calamine`, `rust_xlsxwriter`, `csv`, `serde_json`, `serde_yaml`, `toml`, and `quick-xml` for cross-converting XLSX, XLS, ODS, CSV, TSV, JSON, YAML, TOML, XML, and Markdown tables.
  5. **Document & PDF Engine (Sidecar + Native):** Bundle `pandoc-x86_64-pc-windows-msvc.exe` + `typst-x86_64-pc-windows-msvc.exe` (`--pdf-engine=typst`) for DOCX, MD, HTML, TXT, RTF, EPUB, LaTeX, and PDF conversions, plus `pdfium-render` for PDF-to-Image page extraction.
  6. **Archive Engine (Native Rust):** Use `zip`, `tar`, `flate2`, `sevenz-rust2`, and `xz2` for ZIP, 7Z, TAR, GZ, TGZ, and XZ repacking and extraction.

## 2. UI/UX Rules (Obsidian Dark Minimalist)
- **Color Palette:** `#09090B` background, `#121215` card surface, `#1E1E24` 1px borders, `#EDEDEF` primary text, `#8E8E93` muted text, `#6366F1` indigo accent, `#10B981` success emerald.
- **Frameless Window:** Set `"decorations": false` in `tauri.conf.json`. Build a sleek custom titlebar using `@tauri-apps/api/window` with native drag region and minimize/maximize/close controls.
- **3-Click Workflow:**
  1. Full-window drag-and-drop zone (using Tauri's native file drop events for instant local file paths).
  2. Smart Format Pill Selector: Auto-detects input file category and displays a clean searchable popover with ONLY valid target formats for that file. Includes an "Apply to all [ext] files" one-click action.
  3. Real-Time Queue: Each row shows input badge, target dropdown, optional gear icon (Quality/GPU preset), live percentage progress bar streamed via Tauri Events, and "Show in Explorer" button on completion.
- **Zero Clutter:** Hide advanced sliders (bitrate, resolution, DPI, sample rate) inside a clean per-item popover drawer only when the user clicks the gear icon.

## 3. Coding & Verification Standards
- Keep all TypeScript and Rust files modular and under 200 lines per file.
- Manage child sidecar processes cleanly: store active PIDs in Rust state so clicking "Cancel" or closing the app kills running `ffmpeg`/`pandoc` processes without leaving orphan processes on Windows.
- Verify every phase with `npm run typecheck` and `cargo check` inside `src-tauri`.
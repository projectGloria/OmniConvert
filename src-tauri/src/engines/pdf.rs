//! PDF engine (pdfium): page rasterization and text-layer extraction.
//! Pandoc can't read PDF, so this is everything PDF-as-input honestly supports.

use std::fs::{self, File};
use std::io::{self, BufWriter, Write};
use std::path::Path;
use std::sync::{Mutex, OnceLock, PoisonError};

use image::codecs::jpeg::JpegEncoder;
use image::ImageFormat;
use pdfium_render::prelude::*;

use super::error::{EngineError, EngineResult};
use super::{ConvertSpec, JobCtx};
use crate::registry::{Format, TargetKind};

const DEFAULT_DPI: u32 = 150;

/// One pdfium instance for the whole process. pdfium-render keeps its bindings in a
/// global OnceCell and `bind_to_library` *errors* (AlreadyInitialized) on every call
/// after the first — binding per job made every PDF job after the first one fail.
static PDFIUM: OnceLock<Pdfium> = OnceLock::new();
/// Serializes the first bind. Not `OnceLock::get_or_try_init` (unstable): a failed load
/// (dll missing mid-session) must stay retryable instead of being cached forever.
static BIND: Mutex<()> = Mutex::new(());

fn pdfium(dll: &Path) -> EngineResult<&'static Pdfium> {
    if let Some(p) = PDFIUM.get() {
        return Ok(p); // hot path: no lock once bound
    }
    let _guard = BIND.lock().unwrap_or_else(PoisonError::into_inner);
    if let Some(p) = PDFIUM.get() {
        return Ok(p); // another job bound it while we waited
    }
    let bindings = Pdfium::bind_to_library(dll).map_err(|e| io::Error::other(format!("pdfium.dll failed to load: {e}")))?;
    // `thread_safe` (a default feature) makes Pdfium Send + Sync: calls are serialized
    // behind pdfium-render's own mutex, which pdfium (not thread-safe itself) requires.
    Ok(PDFIUM.get_or_init(|| Pdfium::new(bindings)))
}

pub fn convert(spec: &ConvertSpec, ctx: &JobCtx) -> EngineResult<()> {
    let doc = pdfium(ctx.tools.pdfium()?)?.load_pdf_from_file(&spec.input, None).map_err(EngineError::decode)?;
    let pages = doc.pages();
    let count = (pages.len() as f32).max(1.0);

    match spec.target {
        TargetKind::Convert(Format::Txt) => {
            let mut w = BufWriter::new(File::create(&spec.output)?);
            for (i, page) in pages.iter().enumerate() {
                ctx.check()?;
                let text = page.text().map_err(EngineError::decode)?.all();
                // Form feed between pages: the traditional plain-text page break.
                if i > 0 {
                    w.write_all(b"\n\x0c\n")?;
                }
                w.write_all(text.as_bytes())?;
                ctx.progress.report(Some((i + 1) as f32 / count), "Extracting text");
            }
            w.flush()?;
        }
        TargetKind::Convert(fmt @ (Format::Png | Format::Jpg)) => {
            fs::create_dir_all(&spec.output)?;
            let dpi = spec.settings.dpi.unwrap_or(DEFAULT_DPI);
            // PDF user space is 72 units/inch, so DPI/72 is the render scale.
            let config = PdfRenderConfig::new().scale_page_by_factor(dpi as f32 / 72.0);
            for (i, page) in pages.iter().enumerate() {
                ctx.check()?;
                let img = page
                    .render_with_config(&config)
                    .and_then(|bmp| bmp.as_image())
                    .map_err(EngineError::decode)?;
                let path = spec.output.join(format!("page-{:03}.{}", i + 1, fmt.ext()));
                let mut w = BufWriter::new(File::create(&path)?);
                match fmt {
                    Format::Png => img.write_to(&mut w, ImageFormat::Png).map_err(EngineError::encode)?,
                    _ => JpegEncoder::new_with_quality(&mut w, spec.settings.quality.unwrap_or(90))
                        .encode_image(&img.to_rgb8())
                        .map_err(EngineError::encode)?,
                }
                w.flush()?;
                ctx.progress.report(Some((i + 1) as f32 / count), "Rendering pages");
            }
        }
        other => return Err(EngineError::Unsupported(format!("PDF can't become {}", other.id()))),
    }
    Ok(())
}

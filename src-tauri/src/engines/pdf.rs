//! PDF engine (pdfium): page rasterization and text-layer extraction.
//! Pandoc can't read PDF, so this is everything PDF-as-input honestly supports.

use std::fs::{self, File};
use std::io::{BufWriter, Write};

use image::codecs::jpeg::JpegEncoder;
use image::ImageFormat;
use pdfium_render::prelude::*;

use super::error::{EngineError, EngineResult};
use super::{ConvertSpec, JobCtx};
use crate::registry::{Format, TargetKind};

const DEFAULT_DPI: u32 = 150;

pub fn convert(spec: &ConvertSpec, ctx: &JobCtx) -> EngineResult<()> {
    // pdfium-render keeps the bound library in a process-wide OnceLock, so binding
    // per job is cheap after the first call.
    let bindings = Pdfium::bind_to_library(ctx.tools.pdfium()?).map_err(EngineError::decode)?;
    let pdfium = Pdfium::new(bindings);
    let doc = pdfium.load_pdf_from_file(&spec.input, None).map_err(EngineError::decode)?;
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

//! "Merge to PDF": N images → one PDF, one page per image, in the given order.

use std::path::{Path, PathBuf};

use image::imageops::FilterType;

use super::pdf::{PdfBuilder, PdfImage};
use super::{decode, jpeg_passthrough, svg};
use crate::engines::error::EngineResult;
use crate::engines::JobCtx;
use crate::queue::job::JobSettings;
use crate::registry::Format;

/// Formats the native pipeline decodes without a sidecar. AVIF is excluded on purpose:
/// it needs an ffmpeg decode pass per page, which doesn't belong in a CPU-pool job.
pub const MERGEABLE: [Format; 8] =
    [Format::Png, Format::Jpg, Format::Webp, Format::Bmp, Format::Tiff, Format::Gif, Format::Ico, Format::Svg];

pub fn merge_pdf(inputs: &[(PathBuf, Format)], settings: &JobSettings, out: &Path, ctx: &JobCtx) -> EngineResult<()> {
    let total = inputs.len().max(1) as f32;
    let mut pdf = PdfBuilder::new();

    for (i, (path, fmt)) in inputs.iter().enumerate() {
        ctx.check()?;
        // 0..0.95 for pages; the final save gets the last sliver.
        ctx.progress.report(Some(i as f32 / total * 0.95), &format!("Page {} of {}", i + 1, inputs.len()));
        pdf.add(page(path, *fmt, settings)?)?;
    }

    ctx.check()?;
    ctx.progress.report(Some(0.97), "Writing PDF");
    pdf.save(out)
}

fn page(path: &Path, fmt: Format, settings: &JobSettings) -> EngineResult<PdfImage> {
    // Untouched JPEGs go in byte-for-byte unless a resize was requested.
    if fmt == Format::Jpg && settings.max_dimension.is_none() {
        if let Some(raw) = jpeg_passthrough(path) {
            return Ok(raw);
        }
    }
    let mut img = if fmt == Format::Svg { svg::rasterize(path, settings, None)? } else { decode(path)? };
    if let Some(max) = settings.max_dimension {
        if img.width() > max || img.height() > max {
            img = img.resize(max, max, FilterType::Lanczos3);
        }
    }
    Ok(PdfImage::Raster(img))
}

//! Native image engine: decode (image / resvg) → optional resize → encode.

mod encode;
pub mod merge;
pub mod pdf;
mod svg;

use std::fs;
use std::io::Cursor;
use std::path::Path;

use image::imageops::FilterType;
use image::{DynamicImage, ImageDecoder, ImageReader};

use super::error::{EngineError, EngineResult};
use super::{ConvertSpec, Ctx};
use crate::registry::{Format, TargetKind};
use pdf::PdfImage;

pub fn convert(spec: &ConvertSpec, ctx: &Ctx) -> EngineResult<()> {
    let TargetKind::Convert(target) = spec.target else {
        return Err(EngineError::Unsupported("not an image conversion".into()));
    };

    ctx.progress.report(None, "Decoding");
    let mut img = match spec.src {
        // ICO targets want a 256px master; SVGs are often authored at 24px.
        Format::Svg => svg::rasterize(&spec.input, &spec.settings, (target == Format::Ico).then_some(256))?,
        _ => decode(&spec.input)?,
    };
    ctx.check()?;

    let mut resized = false;
    if let Some(max) = spec.settings.max_dimension {
        if img.width() > max || img.height() > max {
            ctx.progress.report(Some(0.4), "Resizing");
            img = img.resize(max, max, FilterType::Lanczos3); // keeps aspect ratio
            resized = true;
        }
    }
    ctx.check()?;

    ctx.progress.report(Some(0.6), "Encoding");
    match target {
        Format::Pdf => {
            // Untouched JPEGs are embedded byte-for-byte (DCTDecode): no generation loss.
            let page = if spec.src == Format::Jpg && !resized {
                jpeg_passthrough(&spec.input).unwrap_or(PdfImage::Raster(img))
            } else {
                PdfImage::Raster(img)
            };
            pdf::write(vec![page], &spec.output)
        }
        f => encode::encode(&img, f, &spec.settings, &spec.output),
    }
}

/// Content-sniffed decode (the extension may lie — see registry::detect).
pub fn decode(path: &Path) -> EngineResult<DynamicImage> {
    ImageReader::open(path)?
        .with_guessed_format()?
        .decode()
        .map_err(EngineError::decode)
}

/// Raw JPEG bytes + geometry, if the JPEG is plain gray/RGB. CMYK/YCCK JPEGs need an
/// inverted Decode array in PDF that viewers handle inconsistently, so those return
/// None and get re-encoded as raster instead.
pub fn jpeg_passthrough(path: &Path) -> Option<PdfImage> {
    let bytes = fs::read(path).ok()?;
    let decoder = image::codecs::jpeg::JpegDecoder::new(Cursor::new(&bytes)).ok()?;
    let (width, height) = decoder.dimensions();
    let gray = match decoder.color_type() {
        image::ColorType::L8 => true,
        image::ColorType::Rgb8 => false,
        _ => return None,
    };
    Some(PdfImage::Jpeg { bytes, width, height, gray })
}

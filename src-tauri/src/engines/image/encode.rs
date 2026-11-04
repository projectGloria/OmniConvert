use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

use image::codecs::avif::AvifEncoder;
use image::codecs::ico::{IcoEncoder, IcoFrame};
use image::codecs::jpeg::JpegEncoder;
use image::imageops::{self, FilterType};
use image::{DynamicImage, ExtendedColorType, ImageEncoder, ImageFormat, Rgba, RgbaImage, RgbImage};

use crate::engines::error::{EngineError, EngineResult};
use crate::queue::job::JobSettings;
use crate::registry::Format;

const DEFAULT_QUALITY: u8 = 85;
const ICO_SIZES: [u32; 7] = [16, 24, 32, 48, 64, 128, 256];
/// rav1e speed 0–10. 6 is the usual sweet spot; lower is dramatically slower.
const AVIF_SPEED: u8 = 6;

pub fn encode(img: &DynamicImage, target: Format, settings: &JobSettings, out: &Path) -> EngineResult<()> {
    let quality = settings.quality.unwrap_or(DEFAULT_QUALITY).clamp(1, 100);
    let mut w = BufWriter::new(File::create(out)?);

    match target {
        Format::Png => img.write_to(&mut w, ImageFormat::Png).map_err(EngineError::encode)?,
        Format::Bmp => img.write_to(&mut w, ImageFormat::Bmp).map_err(EngineError::encode)?,
        Format::Tiff => img.write_to(&mut w, ImageFormat::Tiff).map_err(EngineError::encode)?,
        // GIF is palette-based; image quantizes RGBA (NeuQuant) internally.
        Format::Gif => DynamicImage::ImageRgba8(img.to_rgba8())
            .write_to(&mut w, ImageFormat::Gif)
            .map_err(EngineError::encode)?,
        Format::Jpg => JpegEncoder::new_with_quality(&mut w, quality)
            .encode_image(&flatten(img, settings.background_rgb()))
            .map_err(EngineError::encode)?,
        Format::Webp => {
            let rgba = img.to_rgba8();
            let enc = webp::Encoder::from_rgba(&rgba, rgba.width(), rgba.height());
            let mem = if quality >= 100 { enc.encode_lossless() } else { enc.encode(f32::from(quality)) };
            w.write_all(&mem)?;
        }
        Format::Avif => {
            let rgba = img.to_rgba8();
            AvifEncoder::new_with_speed_quality(&mut w, AVIF_SPEED, quality)
                .write_image(rgba.as_raw(), rgba.width(), rgba.height(), ExtendedColorType::Rgba8)
                .map_err(EngineError::encode)?;
        }
        Format::Ico => encode_ico(img, &mut w)?,
        other => return Err(EngineError::Unsupported(format!("{} is not an image output", other.label()))),
    }
    w.flush()?;
    Ok(())
}

/// JPEG has no alpha: composite onto `bg` (white by default) instead of letting
/// transparent pixels collapse to black (what a naive `to_rgb8()` does).
fn flatten(img: &DynamicImage, bg: [u8; 3]) -> RgbImage {
    if !img.color().has_alpha() {
        return img.to_rgb8();
    }
    let rgba = img.to_rgba8();
    RgbImage::from_fn(rgba.width(), rgba.height(), |x, y| {
        let Rgba([r, g, b, a]) = *rgba.get_pixel(x, y);
        let a = u16::from(a);
        let mix = |c: u8, under: u8| ((u16::from(c) * a + u16::from(under) * (255 - a)) / 255) as u8;
        image::Rgb([mix(r, bg[0]), mix(g, bg[1]), mix(b, bg[2])])
    })
}

/// Multi-resolution .ico: every standard size up to the source's longest edge,
/// each letterboxed onto a transparent square (icons must be square).
fn encode_ico(img: &DynamicImage, w: &mut impl Write) -> EngineResult<()> {
    let longest = img.width().max(img.height());
    let sizes: Vec<u32> = ICO_SIZES.iter().copied().filter(|&s| s <= longest.max(16)).collect();

    let mut pngs = Vec::with_capacity(sizes.len());
    for &size in &sizes {
        let fitted = img.resize(size, size, FilterType::Lanczos3).to_rgba8();
        let mut canvas = RgbaImage::new(size, size);
        let (x, y) = ((size - fitted.width()) / 2, (size - fitted.height()) / 2);
        imageops::overlay(&mut canvas, &fitted, i64::from(x), i64::from(y));
        pngs.push(canvas);
    }

    let frames = pngs
        .iter()
        .map(|c| IcoFrame::as_png(c.as_raw(), c.width(), c.height(), ExtendedColorType::Rgba8))
        .collect::<Result<Vec<_>, _>>()
        .map_err(EngineError::encode)?;
    IcoEncoder::new(w).encode_images(&frames).map_err(EngineError::encode)
}

use std::fs;
use std::path::Path;

use image::{DynamicImage, RgbaImage};
use resvg::{tiny_skia, usvg};

use crate::engines::error::{EngineError, EngineResult};
use crate::settings::JobSettings;

/// Hard ceiling so a hostile `width="999999"` SVG can't allocate gigabytes.
const MAX_SIDE: f32 = 16_384.0;

/// Rasterize an SVG. Scale priority: explicit max dimension → `min_side` hint
/// (e.g. 256 for ICO) → DPI (96 = 1:1) → native size.
pub fn rasterize(path: &Path, settings: &JobSettings, min_side: Option<u32>) -> EngineResult<DynamicImage> {
    let data = fs::read(path)?;

    let mut opt = usvg::Options { resources_dir: path.parent().map(Path::to_path_buf), ..Default::default() };
    // System fonts only (offline). Without this, every <text> element silently vanishes.
    opt.fontdb_mut().load_system_fonts();

    let tree = usvg::Tree::from_data(&data, &opt).map_err(EngineError::decode)?;
    let size = tree.size();
    let longest = size.width().max(size.height());

    let mut scale = match (settings.max_dimension, min_side) {
        (Some(max), _) => max as f32 / longest,
        (None, Some(min)) if longest < min as f32 => min as f32 / longest,
        _ => settings.dpi.map_or(1.0, |d| d as f32 / 96.0),
    };
    scale = scale.min(MAX_SIDE / longest);

    let w = (size.width() * scale).ceil().max(1.0) as u32;
    let h = (size.height() * scale).ceil().max(1.0) as u32;
    let mut pixmap = tiny_skia::Pixmap::new(w, h).ok_or_else(|| EngineError::Decode("SVG has zero size".into()))?;
    resvg::render(&tree, tiny_skia::Transform::from_scale(scale, scale), &mut pixmap.as_mut());

    // tiny-skia stores premultiplied alpha; `image` expects straight alpha.
    RgbaImage::from_raw(w, h, pixmap.take_demultiplied())
        .map(DynamicImage::ImageRgba8)
        .ok_or_else(|| EngineError::Decode("SVG raster buffer size mismatch".into()))
}

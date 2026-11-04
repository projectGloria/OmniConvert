//! Which engine executes a given (source, target) pair.
//! Consumed by the Phase 3 job runner; until then only the matrix tests call it.
#![cfg_attr(not(test), allow(dead_code))]

use super::formats::{Category, Format};
use super::targets::TargetKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EngineRoute {
    /// ffmpeg/ffprobe sidecar: video, audio, animated outputs, frame extraction.
    Media,
    /// Native `image` crate (+ `webp`, `ravif`, `ico`, `lopdf` for → PDF).
    Image,
    /// AVIF input: ffmpeg decodes to PNG on a pipe, then the native image path encodes.
    MediaThenImage,
    /// `resvg` rasterization, then the native image path.
    Svg,
    /// `pdfium-render`: page rasterization and text extraction.
    Pdf,
    /// pandoc (+ typst as PDF engine) sidecar.
    Document,
    /// calamine / rust_xlsxwriter / csv / serde formats.
    Data,
    /// zip / tar / flate2 / xz2 / sevenz-rust2.
    Archive,
}

pub fn route_for(src: Format, kind: TargetKind) -> EngineRoute {
    use EngineRoute as R;
    match kind {
        TargetKind::AnimatedGif | TargetKind::AnimatedWebp | TargetKind::FramesZip => R::Media,
        TargetKind::Extract => R::Archive,
        TargetKind::Convert(dst) => match src.category() {
            Category::Video | Category::Audio => R::Media,
            // Animated GIF → MP4/WEBM goes through ffmpeg; GIF → still image stays native.
            Category::Image if dst.category() == Category::Video => R::Media,
            Category::Image if src == Format::Avif => R::MediaThenImage,
            Category::Image if src == Format::Svg => R::Svg,
            Category::Image => R::Image,
            Category::Document if src == Format::Pdf => R::Pdf,
            // Markdown table → data formats is a data-engine job, not pandoc.
            Category::Document if dst.category() == Category::Data => R::Data,
            Category::Document => R::Document,
            Category::Data => R::Data,
            Category::Archive => R::Archive,
        },
    }
}

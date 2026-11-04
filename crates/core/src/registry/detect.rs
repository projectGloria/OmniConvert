use std::path::Path;

use super::formats::{Category, Format};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Detected {
    pub format: Option<Format>,
    /// Set when magic bytes disagreed with the extension (or there was no extension)
    /// and won. The UI shows it as a "detected as JPG" hint.
    pub detected_as: Option<Format>,
}

/// Extension first, magic bytes second.
///
/// Magic bytes only override the extension for *images*, where mislabelled files are
/// common (a `.png` that's really a JPEG from a "save as" rename) and the sniff is
/// unambiguous. For media and ZIP-based containers the extension is the better signal:
/// `infer` would call an audio-only `.mp4` "m4a" and an odd `.docx` plain "zip".
pub fn detect(path: &Path) -> Detected {
    let by_ext = Format::from_path(path);
    let by_magic = infer::get_from_path(path)
        .ok()
        .flatten()
        .and_then(|kind| from_infer(kind.extension()));

    match (by_ext, by_magic) {
        (None, magic) => Detected { format: magic, detected_as: magic },
        (Some(ext), Some(magic))
            if ext != magic
                && ext.category() == Category::Image
                && magic.category() == Category::Image =>
        {
            Detected { format: Some(magic), detected_as: Some(magic) }
        }
        (ext, _) => Detected { format: ext, detected_as: None },
    }
}

/// Map `infer`'s extension strings onto our formats. Text formats (CSV, JSON, MD…)
/// have no magic number, so `infer` returns None for them and the extension wins.
fn from_infer(ext: &str) -> Option<Format> {
    match ext {
        "tif" => Some(Format::Tiff),
        "jpeg" => Some(Format::Jpg),
        // Old CFB containers (.xls/.doc/.msi) all sniff as the same thing; trust the extension.
        "msi" | "cfb" => None,
        other => Format::from_ext(other),
    }
}

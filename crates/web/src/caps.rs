//! What the web build can actually run. The matrix stays the single source of truth
//! (PLAN D1); this only *subtracts* routes whose engine doesn't exist in the browser yet,
//! so the picker never offers a conversion the worker would reject.

use omniconvert_core::registry::{route_for, EngineRoute, Format, Target, TargetKind};

/// Routes with a browser implementation.
/// - `MediaThenImage` (AVIF input): the worker decodes AVIF with the browser's own
///   decoder to PNG, then calls the image engine, replacing the desktop's ffmpeg pass.
/// - Media / Document / Pdf need ffmpeg.wasm / pandoc.wasm / pdf.js (later phases).
pub fn route_supported(route: EngineRoute) -> bool {
    matches!(
        route,
        EngineRoute::Image | EngineRoute::Svg | EngineRoute::MediaThenImage | EngineRoute::Data | EngineRoute::Archive
    )
}

/// Extraction writes a folder tree; browsers download files, so it waits for a
/// folder-output story (File System Access) before it's offered.
pub fn target_supported(src: Format, kind: TargetKind) -> bool {
    !matches!(kind, TargetKind::Extract) && route_supported(route_for(src, kind))
}

pub fn filter(src: Option<Format>, targets: Vec<Target>) -> Vec<Target> {
    match src {
        Some(src) => targets.into_iter().filter(|t| target_supported(src, t.kind)).collect(),
        None => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use omniconvert_core::registry::Format::*;

    #[test]
    fn native_routes_are_offered() {
        assert!(target_supported(Png, TargetKind::Convert(Webp)));
        assert!(target_supported(Avif, TargetKind::Convert(Png)));
        assert!(target_supported(Csv, TargetKind::Convert(Xlsx)));
        assert!(target_supported(Zip, TargetKind::Convert(SevenZ)));
    }

    #[test]
    fn sidecar_routes_and_extract_are_hidden() {
        assert!(!target_supported(Mp4, TargetKind::Convert(Mp3)));
        assert!(!target_supported(Docx, TargetKind::Convert(Pdf)));
        assert!(!target_supported(Pdf, TargetKind::Convert(Png)));
        assert!(!target_supported(Zip, TargetKind::Extract));
    }
}

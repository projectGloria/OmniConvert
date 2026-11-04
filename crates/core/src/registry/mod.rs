//! Universal format registry: what a file is, and what it can become.

mod detect;
mod formats;
mod matrix;
mod route;
mod shape;
mod targets;

#[cfg(test)]
mod tests;

use std::path::Path;

use serde::Serialize;
use ts_rs::TS;

pub use formats::{Category, Format};
pub use route::{route_for, EngineRoute};
pub use shape::rows_of;
pub use targets::{Target, TargetKind};

/// Everything the UI needs to render one queue row. Returned by `probe_files`.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct FileInfo {
    pub path: String,
    pub name: String,
    /// Raw lowercase extension as on disk (badge text), independent of detection.
    pub ext: String,
    // u64 would map to `bigint` in TS; file sizes fit comfortably in f64's 2^53.
    #[ts(type = "number")]
    pub size: u64,
    /// None ⇒ unsupported file: shown in the queue, but with no targets.
    pub format: Option<Format>,
    pub category: Option<Category>,
    pub detected_as: Option<Format>,
    pub targets: Vec<Target>,
}

/// Detect + sniff + resolve targets for a single regular file.
pub fn inspect(path: &Path, size: u64) -> FileInfo {
    let detected = detect::detect(path);
    let targets = detected
        .format
        .map(|f| matrix::targets_for(f, &shape::Traits::sniff(path, f, size)))
        .unwrap_or_default();

    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();

    FileInfo {
        path: path.to_string_lossy().into_owned(),
        ext: display_ext(path, &name),
        name,
        size,
        format: detected.format,
        category: detected.format.map(Format::category),
        detected_as: detected.detected_as,
        targets,
    }
}

/// Badge text: the on-disk extension, except compound archive suffixes where
/// `Path::extension()` would report just "gz" for `backup.tar.gz`.
fn display_ext(path: &Path, name: &str) -> String {
    let lower = name.to_ascii_lowercase();
    for compound in ["tar.gz", "tar.xz"] {
        if lower.ends_with(&format!(".{compound}")) {
            return compound.to_owned();
        }
    }
    path.extension().map(|e| e.to_string_lossy().to_ascii_lowercase()).unwrap_or_default()
}

//! Wire types for the job queue (exported to TS via ts-rs).

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::registry::TargetKind;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum Preset {
    /// GPU constant-quality, speed-biased.
    Fast,
    /// GPU constant-quality, quality-biased.
    #[default]
    Balanced,
    /// Always CPU (libx265 / libx264 CRF): smallest files, slowest.
    Archival,
}

/// Per-item knobs from the settings drawer. Every field is optional: `None` means
/// "engine default", so the frontend sends a sparse object and defaults live in Rust.
#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct JobSettings {
    pub preset: Preset,
    /// 1–100 for JPG/WebP/AVIF. WebP at 100 switches to lossless.
    #[ts(optional)]
    pub quality: Option<u8>,
    /// Longest edge in px for image outputs and GIF/WebP animations.
    #[ts(optional)]
    pub max_dimension: Option<u32>,
    /// Output video height (width follows aspect ratio).
    #[ts(optional)]
    pub video_height: Option<u32>,
    /// kbps for lossy audio targets.
    #[ts(optional)]
    pub audio_bitrate: Option<u32>,
    /// Frame rate for animated outputs / frame extraction.
    #[ts(optional)]
    pub fps: Option<u32>,
    /// Rasterization DPI for PDF pages and SVG.
    #[ts(optional)]
    pub dpi: Option<u32>,
    /// Hz for audio targets (None = keep source rate). Ignored for Opus (48k only).
    #[ts(optional)]
    pub sample_rate: Option<u32>,
    /// "#rrggbb" that transparent pixels are flattened onto for alpha-less targets (JPG).
    #[ts(optional)]
    pub background: Option<String>,
}

impl JobSettings {
    /// Parsed `background`, white when unset or malformed. Malformed is not an error:
    /// the UI only ever sends a colour picker value, so garbage means a stale client.
    pub fn background_rgb(&self) -> [u8; 3] {
        let parse = |hex: &str| -> Option<[u8; 3]> {
            let h = hex.strip_prefix('#')?;
            if h.len() != 6 {
                return None;
            }
            let byte = |i: usize| u8::from_str_radix(h.get(i..i + 2)?, 16).ok();
            Some([byte(0)?, byte(2)?, byte(4)?])
        };
        self.background.as_deref().and_then(parse).unwrap_or([255, 255, 255])
    }
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct JobRequest {
    /// Frontend-generated id (the queue row id) — every event echoes it back.
    pub job_id: String,
    pub path: String,
    pub target: TargetKind,
    /// None ⇒ write next to the source file.
    pub output_dir: Option<String>,
    #[serde(default)]
    pub settings: JobSettings,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CompleteEvent {
    pub job_id: String,
    pub output: String,
    #[ts(type = "number")]
    pub elapsed_ms: u64,
    /// Size of the output (summed for directory outputs) — feeds the batch summary.
    #[ts(type = "number")]
    pub output_bytes: u64,
}

/// "Merge to PDF": many images → one PDF, run as a single queue job.
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct MergeRequest {
    pub job_id: String,
    /// Page order = array order.
    pub paths: Vec<String>,
    pub output_dir: Option<String>,
    #[serde(default)]
    pub settings: JobSettings,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ErrorEvent {
    pub job_id: String,
    pub message: String,
    /// Sidecar stderr tail when a tool failed — shown in an expandable panel, not inline.
    pub detail: Option<String>,
    /// True when the user cancelled — the UI shows "Cancelled", not an error.
    pub cancelled: bool,
}

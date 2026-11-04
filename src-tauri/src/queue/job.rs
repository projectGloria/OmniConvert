//! Wire types for the job queue (exported to TS via ts-rs).

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::registry::TargetKind;
pub use omniconvert_core::settings::{JobSettings, Preset};

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

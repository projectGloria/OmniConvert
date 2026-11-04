use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde::Serialize;
use tauri::{AppHandle, Emitter};
use ts_rs::TS;

pub use omniconvert_core::progress::{NullSink, ProgressSink, Scaled};

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ProgressEvent {
    pub job_id: String,
    pub progress: Option<f32>,
    pub stage: String,
}

const MIN_INTERVAL: Duration = Duration::from_millis(100);
const MIN_DELTA: f32 = 0.005;

/// Throttled emitter: ≤ ~10 events/s per job, and only when progress moved ≥ 0.5%
/// (PLAN D10). ffmpeg's `-progress` output would otherwise flood IPC and make React
/// re-render the progress bar hundreds of times a second across parallel jobs.
pub struct TauriSink {
    app: AppHandle,
    job_id: String,
    last: Mutex<(Option<Instant>, f32, String)>,
}

impl TauriSink {
    pub fn new(app: AppHandle, job_id: String) -> Self {
        Self { app, job_id, last: Mutex::new((None, -1.0, String::new())) }
    }
}

impl ProgressSink for TauriSink {
    fn report(&self, fraction: Option<f32>, stage: &str) {
        let Ok(mut last) = self.last.lock() else { return };
        let (at, value, last_stage) = &mut *last;
        let now = Instant::now();
        let f = fraction.unwrap_or(-1.0);

        // Stage changes always go through (they're rare and the user should see them);
        // otherwise require both a time gap and a meaningful delta.
        let stage_changed = *last_stage != stage;
        let due = at.is_none_or(|t| now.duration_since(t) >= MIN_INTERVAL);
        if !stage_changed && !(due && (f - *value).abs() >= MIN_DELTA) {
            return;
        }

        *at = Some(now);
        *value = f;
        stage.clone_into(last_stage);
        let _ = self.app.emit(
            "conversion://progress",
            ProgressEvent { job_id: self.job_id.clone(), progress: fraction, stage: stage.to_owned() },
        );
    }
}

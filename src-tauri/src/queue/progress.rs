use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde::Serialize;
use tauri::{AppHandle, Emitter};
use ts_rs::TS;

/// Engines report progress through this trait, never through Tauri directly —
/// that's what lets `cargo test` drive converters without an app (PLAN D9).
pub trait ProgressSink: Send + Sync {
    /// `fraction` in 0..=1, or None for indeterminate work (pandoc, tar streams).
    fn report(&self, fraction: Option<f32>, stage: &str);
}

/// Swallows everything. Used by engine tests and the e2e testkit — not cfg(test)-gated
/// because `tests/` links the lib as a normal (non-test) build.
pub struct NullSink;
impl ProgressSink for NullSink {
    fn report(&self, _: Option<f32>, _: &str) {}
}

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

/// Maps a sub-step's 0..1 progress into a slice [start, end] of the parent's range.
/// E.g. frames→ZIP: ffmpeg extraction is 0..0.9, zipping is 0.9..1.
pub struct Scaled<'a> {
    pub inner: &'a dyn ProgressSink,
    pub start: f32,
    pub end: f32,
}

impl ProgressSink for Scaled<'_> {
    fn report(&self, fraction: Option<f32>, stage: &str) {
        self.inner.report(fraction.map(|f| self.start + f * (self.end - self.start)), stage);
    }
}

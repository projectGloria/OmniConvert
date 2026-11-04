//! Progress → stderr, one JSON object per line. The worker's line-buffered stderr fd
//! turns each line into a postMessage, which the main thread re-emits as the same
//! `conversion://progress` event the desktop build gets from Tauri.

use std::io::Write;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use omniconvert_core::progress::ProgressSink;
use serde::Serialize;

const MIN_INTERVAL: Duration = Duration::from_millis(100);
const MIN_DELTA: f32 = 0.005;

#[derive(Serialize)]
struct Line<'a> {
    progress: Option<f32>,
    stage: &'a str,
}

/// Same throttle as the desktop's TauriSink (PLAN D10): ≤ ~10 lines/s, only on a
/// ≥0.5% move or a stage change. Each line crosses the wasm→JS boundary *and* a
/// postMessage, so the cap matters just as much here.
#[derive(Default)]
pub struct StderrSink {
    last: Mutex<(Option<Instant>, f32, String)>,
}

impl ProgressSink for StderrSink {
    fn report(&self, fraction: Option<f32>, stage: &str) {
        let Ok(mut last) = self.last.lock() else { return };
        let (at, value, last_stage) = &mut *last;
        let now = Instant::now();
        let f = fraction.unwrap_or(-1.0);

        let stage_changed = *last_stage != stage;
        let due = at.is_none_or(|t| now.duration_since(t) >= MIN_INTERVAL);
        if !stage_changed && !(due && (f - *value).abs() >= MIN_DELTA) {
            return;
        }
        *at = Some(now);
        *value = f;
        stage.clone_into(last_stage);

        if let Ok(json) = serde_json::to_string(&Line { progress: fraction, stage }) {
            // A failed progress write must never fail the conversion itself.
            let _ = writeln!(std::io::stderr(), "{json}");
        }
    }
}

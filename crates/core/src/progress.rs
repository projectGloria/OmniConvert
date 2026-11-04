//! Progress reporting seam. Engines report through `ProgressSink`, never through a
//! host API — the desktop implements it with throttled Tauri events, the web engine
//! with throttled stderr lines that the worker forwards via postMessage.

/// `fraction` in 0..=1, or None for indeterminate work (pandoc, tar streams).
pub trait ProgressSink: Send + Sync {
    fn report(&self, fraction: Option<f32>, stage: &str);
}

/// Swallows everything. Used by engine tests and the e2e testkit.
pub struct NullSink;
impl ProgressSink for NullSink {
    fn report(&self, _: Option<f32>, _: &str) {}
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

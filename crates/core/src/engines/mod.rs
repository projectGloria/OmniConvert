//! Pure-Rust conversion engines. Sidecar engines (ffmpeg, pandoc, pdfium) live in the
//! desktop shell; the web shell substitutes browser/WASM equivalents for them.

pub mod archive;
pub mod data;
pub mod error;
pub mod image;
pub mod util;

#[cfg(test)]
mod tests;

use std::path::PathBuf;
use std::sync::Arc;

use tokio_util::sync::CancellationToken;

use crate::progress::ProgressSink;
use crate::registry::{Format, TargetKind};
use crate::settings::JobSettings;
use error::{EngineError, EngineResult};

/// One fully-resolved conversion. `output` is the path to write (the desktop queue
/// passes its *part* path; the web engine passes an in-memory file).
#[derive(Debug, Clone)]
pub struct ConvertSpec {
    pub input: PathBuf,
    pub src: Format,
    pub target: TargetKind,
    pub output: PathBuf,
    pub settings: JobSettings,
}

/// What a native engine needs from its host: a cancel flag and somewhere to report.
/// Owned (Arc/clone) so the desktop can move it into `spawn_blocking` ('static).
#[derive(Clone)]
pub struct Ctx {
    pub cancel: CancellationToken,
    pub progress: Arc<dyn ProgressSink>,
}

impl Ctx {
    /// Cooperative cancellation point for CPU-bound engines.
    pub fn check(&self) -> EngineResult<()> {
        if self.cancel.is_cancelled() {
            Err(EngineError::Cancelled)
        } else {
            Ok(())
        }
    }
}

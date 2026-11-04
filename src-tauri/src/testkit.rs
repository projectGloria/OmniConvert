//! Narrow public surface for the `tests/e2e_routes` suite. Not an API (`doc(hidden)`).
//!
//! Integration tests link this crate like any outside user, so private modules are
//! invisible to them. Making the whole engine tree `pub` would also blind rustc's
//! dead-code lint for every item in it; re-exporting just what a route test needs keeps
//! the lint honest everywhere else.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use tokio_util::sync::CancellationToken;

use crate::engines::{self, ConvertSpec, JobCtx};
use crate::gpu::GpuCaps;
use crate::queue::output;
use crate::queue::progress::NullSink;
use crate::sidecar::Tools;

pub use crate::engines::error::EngineError;
pub use crate::queue::job::{JobSettings, Preset};
pub use crate::registry::{FileInfo, Format, TargetKind};

/// Same detection + target resolution the UI gets from `probe_files`.
pub fn inspect(path: &Path) -> FileInfo {
    let size = fs::metadata(path).map_or(0, |m| m.len());
    crate::registry::inspect(path, size)
}

/// Drives one conversion the way `JobQueue` does (re-detect → matrix check → plan a
/// part path → engine → atomic rename), minus Tauri, pools and events (PLAN D9).
pub struct Harness {
    tools: Arc<Tools>,
    // One runtime per harness: engines need Tokio for `spawn_blocking` and child-process
    // pipes, and `block_on` keeps each test body plain synchronous code.
    rt: tokio::runtime::Runtime,
}

impl Harness {
    pub fn new() -> Self {
        let rt = tokio::runtime::Builder::new_multi_thread().worker_threads(2).enable_all().build().expect("tokio runtime");
        // Debug builds fall back to src-tauri/binaries + resources (sidecar::paths), so
        // tests use exactly the binaries `npm run sidecars` fetched for `tauri dev`.
        Self { tools: Arc::new(Tools::discover(None)), rt }
    }

    /// Converts `input` next to itself and returns the final output path. Fails with
    /// `Unsupported` if the compatibility matrix doesn't offer `target` for this file,
    /// so a route test also proves the picker would have shown it.
    pub fn convert(&self, input: &Path, target: TargetKind, settings: JobSettings) -> Result<PathBuf, EngineError> {
        let info = inspect(input);
        let src = info.format.ok_or_else(|| EngineError::Unsupported(format!("undetected: {}", input.display())))?;
        if !info.targets.iter().any(|t| t.kind == target) {
            return Err(EngineError::Unsupported(format!("matrix doesn't offer {:?} → {}", src, target.id())));
        }

        let out = output::plan(input, src, target, None, |_| false);
        let ctx = JobCtx {
            cancel: CancellationToken::new(),
            progress: Arc::new(NullSink),
            tools: Arc::clone(&self.tools),
            gpu: GpuCaps::default(), // CPU encoders: deterministic on any machine
        };
        let spec = ConvertSpec { input: input.to_path_buf(), src, target, output: out.part_path.clone(), settings };
        match self.rt.block_on(engines::execute(spec, ctx)) {
            Ok(()) => Ok(output::finalize(&out)?),
            Err(e) => {
                output::discard(&out);
                Err(e)
            }
        }
    }
}

impl Default for Harness {
    fn default() -> Self {
        Self::new()
    }
}

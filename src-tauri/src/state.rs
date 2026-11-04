use std::path::Path;
use std::sync::{Arc, RwLock};

use crate::gpu::GpuCaps;
use crate::queue::journal::Journal;
use crate::queue::JobQueue;
use crate::sidecar::Tools;

/// Managed Tauri state. Everything is Arc'd so commands can clone handles into
/// spawned tasks without holding the State guard across awaits.
pub struct AppState {
    pub queue: Arc<JobQueue>,
    pub tools: Arc<Tools>,
    /// Filled asynchronously by the startup GPU probe.
    pub gpu: Arc<RwLock<GpuCaps>>,
}

impl AppState {
    /// `data_dir`: app-local data folder for the crash journal (swept on construction).
    pub fn new(tools: Tools, data_dir: Option<&Path>) -> Self {
        Self { queue: Arc::new(JobQueue::new(Journal::open(data_dir))), tools: Arc::new(tools), gpu: Arc::default() }
    }

    pub fn gpu_snapshot(&self) -> GpuCaps {
        self.gpu.read().map(|g| g.clone()).unwrap_or_default()
    }
}

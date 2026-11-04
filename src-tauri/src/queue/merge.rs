//! "Merge to PDF" group job: many images in, one PDF out, one queue row.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use tauri::AppHandle;

use super::job::MergeRequest;
use super::output;
use super::runner::{emit_error, Job, JobQueue};
use crate::engines;
use crate::engines::image::merge::MERGEABLE;
use crate::gpu::GpuCaps;
use crate::registry::{self, Format};
use crate::sidecar::Tools;

/// A runaway selection shouldn't build a 5 GB PDF by accident.
const MAX_PAGES: usize = 500;

impl JobQueue {
    pub fn enqueue_merge(self: &Arc<Self>, app: AppHandle, req: MergeRequest, tools: Arc<Tools>, gpu: GpuCaps) {
        let inputs = match validate(&req.paths) {
            Ok(v) => v,
            Err(message) => return emit_error(&app, &req.job_id, message, None, false),
        };
        let first = inputs[0].0.clone(); // validate() guarantees non-empty
        let MergeRequest { job_id, output_dir, settings, .. } = req;
        let pool = Arc::clone(&self.cpu); // all-native decode + Flate: CPU pool
        self.spawn(
            app,
            Job {
                id: job_id,
                pool,
                plan: Box::new(move |taken| output::plan_merge(&first, output_dir.as_deref().map(Path::new), taken)),
                work: Box::new(move |out, ctx| Box::pin(engines::merge_pdf(inputs, settings, out, ctx))),
                tools,
                gpu,
            },
        );
    }
}

/// Same rule as single jobs: the frontend's view is a hint, Rust re-detects every file.
fn validate(paths: &[String]) -> Result<Vec<(PathBuf, Format)>, String> {
    if paths.is_empty() {
        return Err("nothing to merge".into());
    }
    if paths.len() > MAX_PAGES {
        return Err(format!("merge is limited to {MAX_PAGES} images"));
    }
    paths
        .iter()
        .map(|p| {
            let path = PathBuf::from(p);
            let size = fs::metadata(&path).map_err(|e| format!("can't read {p}: {e}"))?.len();
            match registry::inspect(&path, size).format {
                Some(f) if MERGEABLE.contains(&f) => Ok((path, f)),
                Some(f) => Err(format!("{} files can't be merged into a PDF", f.label())),
                None => Err(format!("unsupported file: {p}")),
            }
        })
        .collect()
}

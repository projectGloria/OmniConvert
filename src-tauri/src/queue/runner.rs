//! The job queue: validation, pool scheduling, cancellation, lifecycle events.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use tauri::{AppHandle, Emitter};
use tokio::sync::Semaphore;
use tokio_util::sync::CancellationToken;

use super::job::{CompleteEvent, ErrorEvent, JobRequest};
use super::output;
use super::progress::{ProgressSink, TauriSink};
use crate::engines::error::EngineError;
use crate::engines::{self, ConvertSpec, JobCtx};
use crate::gpu::GpuCaps;
use crate::registry::{self, route_for, EngineRoute, Format, TargetKind};
use crate::sidecar::Tools;

pub struct JobQueue {
    /// Parent of every job token: `cancel_all` is one call, then it's replaced so
    /// jobs enqueued afterwards don't start life already cancelled.
    root: Mutex<CancellationToken>,
    tokens: Mutex<HashMap<String, CancellationToken>>,
    /// Final output paths claimed by in-flight jobs. Two queued files `a.png` and
    /// `a.jpg` → WebP must not both pick `a.webp` because neither exists *yet*.
    reserved: Mutex<HashSet<PathBuf>>,
    /// Pure-Rust work: image/data/archive. Cores/2 leaves room for the UI + sidecars.
    cpu: Arc<Semaphore>,
    /// CPU ffmpeg/pandoc. x264/x265 already use every core; more parallelism just thrashes.
    sidecar: Arc<Semaphore>,
    /// Hardware encodes: consumer NVENC caps concurrent sessions per GPU.
    gpu: Arc<Semaphore>,
}

impl JobQueue {
    pub fn new() -> Self {
        let cores = std::thread::available_parallelism().map_or(4, |n| n.get());
        Self {
            root: Mutex::new(CancellationToken::new()),
            tokens: Mutex::default(),
            reserved: Mutex::default(),
            cpu: Arc::new(Semaphore::new((cores / 2).max(1))),
            sidecar: Arc::new(Semaphore::new(2)),
            gpu: Arc::new(Semaphore::new(2)),
        }
    }

    pub fn cancel(&self, job_id: &str) {
        if let Some(t) = self.tokens.lock().expect("tokens lock").get(job_id) {
            t.cancel();
        }
    }

    pub fn cancel_all(&self) {
        let mut root = self.root.lock().expect("root lock");
        root.cancel();
        *root = CancellationToken::new();
    }

    pub fn enqueue(self: &Arc<Self>, app: AppHandle, req: JobRequest, tools: Arc<Tools>, gpu: GpuCaps) {
        // Re-validate in Rust: the frontend's target list could be stale (file changed
        // on disk since it was probed) and the matrix is the authority anyway.
        let (src, input) = match self.validate(&req) {
            Ok(v) => v,
            Err(message) => return emit_error(&app, &req.job_id, message, false),
        };

        let token = self.root.lock().expect("root lock").child_token();
        self.tokens.lock().expect("tokens lock").insert(req.job_id.clone(), token.clone());
        let pool = self.pool_for(src, req.target, &gpu);
        let queue = Arc::clone(self);

        tauri::async_runtime::spawn(async move {
            let id = req.job_id.clone();
            // Waiting for a permit is cancellable: Cancel on a queued row is instant.
            let permit = tokio::select! {
                p = pool.acquire_owned() => p.ok(),
                () = token.cancelled() => None,
            };
            if permit.is_some() {
                queue.run(&app, req, src, input, token, tools, gpu).await;
            } else {
                emit_error(&app, &id, "cancelled".into(), true);
            }
            queue.tokens.lock().expect("tokens lock").remove(&id);
        });
    }

    fn validate(&self, req: &JobRequest) -> Result<(Format, PathBuf), String> {
        let input = PathBuf::from(&req.path);
        let size = fs::metadata(&input).map_err(|e| format!("can't read source: {e}"))?.len();
        let info = registry::inspect(&input, size);
        let src = info.format.ok_or("unsupported file type")?;
        if !info.targets.iter().any(|t| t.kind == req.target) {
            return Err(format!("{} can't be converted to {}", src.label(), req.target.id()));
        }
        Ok((src, input))
    }

    fn pool_for(&self, src: Format, target: TargetKind, gpu: &GpuCaps) -> Arc<Semaphore> {
        let p = match route_for(src, target) {
            EngineRoute::Media => {
                let video_out = matches!(target, TargetKind::Convert(f) if f.category() == registry::Category::Video);
                if video_out && !gpu.encoders.is_empty() { &self.gpu } else { &self.sidecar }
            }
            EngineRoute::Document | EngineRoute::MediaThenImage => &self.sidecar,
            _ => &self.cpu,
        };
        Arc::clone(p)
    }

    #[allow(clippy::too_many_arguments)]
    async fn run(&self, app: &AppHandle, req: JobRequest, src: Format, input: PathBuf, token: CancellationToken, tools: Arc<Tools>, gpu: GpuCaps) {
        let started = Instant::now();
        let sink: Arc<dyn ProgressSink> = Arc::new(TauriSink::new(app.clone(), req.job_id.clone()));
        sink.report(None, "Starting");

        let out = {
            let mut reserved = self.reserved.lock().expect("reserved lock");
            let planned = output::plan(&input, src, req.target, req.output_dir.as_deref().map(Path::new), |p| reserved.contains(p));
            match planned {
                Ok(o) => {
                    reserved.insert(o.final_path.clone());
                    o
                }
                Err(e) => return emit_error(app, &req.job_id, e.to_string(), false),
            }
        };

        let spec = ConvertSpec { input, src, target: req.target, output: out.part_path.clone(), settings: req.settings };
        let ctx = JobCtx { cancel: token.clone(), progress: sink, tools, gpu };
        // Inner spawn = panic isolation: a panicking engine becomes a JoinError here
        // instead of silently killing this task and leaving the row "running" forever.
        let result = match tauri::async_runtime::spawn(engines::execute(spec, ctx)).await {
            Ok(r) => r,
            Err(e) => Err(EngineError::Encode(format!("converter crashed: {e}"))),
        };
        let result = result.and_then(|()| output::finalize(&out).map_err(EngineError::from));
        self.reserved.lock().expect("reserved lock").remove(&out.final_path);

        match result {
            Ok(path) => {
                let _ = app.emit(
                    "conversion://complete",
                    CompleteEvent {
                        job_id: req.job_id,
                        output: path.to_string_lossy().into_owned(),
                        elapsed_ms: started.elapsed().as_millis() as u64,
                    },
                );
            }
            Err(e) => {
                output::discard(&out);
                // Library errors raised *because* we cancelled (CancelReader) still count as cancel.
                let cancelled = matches!(e, EngineError::Cancelled) || token.is_cancelled();
                emit_error(app, &req.job_id, if cancelled { "cancelled".into() } else { e.to_string() }, cancelled);
            }
        }
    }
}

fn emit_error(app: &AppHandle, job_id: &str, message: String, cancelled: bool) {
    let _ = app.emit("conversion://error", ErrorEvent { job_id: job_id.to_owned(), message, cancelled });
}

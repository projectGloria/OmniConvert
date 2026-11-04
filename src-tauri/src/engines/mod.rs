//! Desktop engine routing. Pure-Rust engines come from `omniconvert-core` (shared with
//! the web build); sidecar engines (ffmpeg, pandoc, pdfium) live here (PLAN D9 still
//! holds: none of them know about Tauri, so `cargo test` can drive them).

pub mod document;
pub mod media;
pub mod pdf;

// Re-exported so `crate::engines::image::…` etc. keep resolving after the core split.
pub use omniconvert_core::engines::{archive, data, error, image, util, ConvertSpec};

use std::path::PathBuf;
use std::sync::Arc;

use tokio_util::sync::CancellationToken;

use crate::gpu::GpuCaps;
use crate::queue::job::JobSettings;
use crate::queue::progress::ProgressSink;
use crate::registry::{route_for, EngineRoute, Format};
use crate::sidecar::Tools;
use error::{EngineError, EngineResult};
use omniconvert_core::engines::Ctx;

/// Owned (Arc/clone) so it can cross into `spawn_blocking`, which demands 'static.
#[derive(Clone)]
pub struct JobCtx {
    pub cancel: CancellationToken,
    pub progress: Arc<dyn ProgressSink>,
    pub tools: Arc<Tools>,
    pub gpu: GpuCaps,
}

impl JobCtx {
    /// Cooperative cancellation point for CPU-bound engines.
    pub fn check(&self) -> EngineResult<()> {
        if self.cancel.is_cancelled() {
            Err(EngineError::Cancelled)
        } else {
            Ok(())
        }
    }

    /// The slice of this context the core (pure-Rust) engines need.
    pub fn native(&self) -> Ctx {
        Ctx { cancel: self.cancel.clone(), progress: Arc::clone(&self.progress) }
    }
}

/// Route a job to its engine. Sidecar engines are async (they await processes);
/// pure-Rust engines are CPU-bound and go to the blocking pool so they never stall
/// the Tokio workers that drive IPC and the other jobs' process pipes.
pub async fn execute(spec: ConvertSpec, ctx: JobCtx) -> EngineResult<()> {
    match route_for(spec.src, spec.target) {
        EngineRoute::Media => media::convert(&spec, &ctx).await,
        EngineRoute::Document => document::convert(&spec, &ctx).await,
        EngineRoute::MediaThenImage => {
            // AVIF decode via ffmpeg → temp PNG → native image pipeline (PLAN D7).
            let png = spec.output.with_extension("omni-decode.png");
            media::decode_still(&spec.input, &png, &ctx).await?;
            let next = ConvertSpec { input: png.clone(), src: Format::Png, ..spec };
            let result = blocking(ctx, move |c| image::convert(&next, &c.native())).await;
            let _ = std::fs::remove_file(&png);
            result
        }
        EngineRoute::Image | EngineRoute::Svg => blocking(ctx, move |c| image::convert(&spec, &c.native())).await,
        EngineRoute::Pdf => blocking(ctx, move |c| pdf::convert(&spec, &c)).await,
        EngineRoute::Data => blocking(ctx, move |c| data::convert(&spec, &c.native())).await,
        EngineRoute::Archive => blocking(ctx, move |c| archive::convert(&spec, &c.native())).await,
    }
}

/// "Merge to PDF" (one job, many inputs) — see engines::image::merge.
pub async fn merge_pdf(inputs: Vec<(PathBuf, Format)>, settings: JobSettings, out: PathBuf, ctx: JobCtx) -> EngineResult<()> {
    blocking(ctx, move |c| image::merge::merge_pdf(&inputs, &settings, &out, &c.native())).await
}

async fn blocking<F>(ctx: JobCtx, f: F) -> EngineResult<()>
where
    F: FnOnce(JobCtx) -> EngineResult<()> + Send + 'static,
{
    tokio::task::spawn_blocking(move || f(ctx))
        .await
        // A JoinError here means the converter panicked (e.g. a decoder bug on a hostile file).
        .unwrap_or_else(|e| Err(EngineError::Encode(format!("converter crashed: {e}"))))
}

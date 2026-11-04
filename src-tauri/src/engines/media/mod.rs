//! Media engine (ffmpeg/ffprobe sidecars).

mod args;

use std::fs::{self, File};
use std::io::{self, BufWriter};
use std::path::Path;

use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

use super::error::{EngineError, EngineResult};
use super::util::path_arg;
use super::{ConvertSpec, JobCtx};
use crate::gpu::GpuCaps;
use crate::queue::progress::{ProgressSink, Scaled};
use crate::registry::TargetKind;
use crate::sidecar::{self, RunOptions};

pub async fn convert(spec: &ConvertSpec, ctx: &JobCtx) -> EngineResult<()> {
    let duration = probe_duration(spec, ctx).await;

    if spec.target == TargetKind::FramesZip {
        return frames_zip(spec, ctx, duration).await;
    }

    let plan = args::build(spec, &ctx.gpu)?;
    let first = run_ffmpeg(&plan.args, ctx, duration, &*ctx.progress).await;
    match first {
        // GPU encoders fail for reasons a probe can't foresee (driver resets, NVENC
        // session caps, unsupported profile for this input). Retry once on CPU
        // rather than surfacing an error the user can't act on.
        Err(EngineError::Tool { .. }) if plan.uses_gpu => {
            ctx.progress.report(Some(0.0), "GPU encode failed — retrying on CPU");
            let cpu = args::build(spec, &GpuCaps { ready: true, encoders: vec![] })?;
            run_ffmpeg(&cpu.args, ctx, duration, &*ctx.progress).await
        }
        other => other,
    }
}

/// AVIF → PNG via ffmpeg's libdav1d decoder (PLAN D7).
pub async fn decode_still(input: &Path, png: &Path, ctx: &JobCtx) -> EngineResult<()> {
    let a = [
        "-hide_banner".into(), "-nostdin".into(), "-y".into(), "-v".into(), "error".into(),
        "-i".into(), path_arg(input), "-frames:v".into(), "1".into(), path_arg(png),
    ];
    ctx.progress.report(None, "Decoding AVIF");
    sidecar::run(ctx.tools.ffmpeg()?, a, RunOptions { cwd: None, cancel: &ctx.cancel }, |_| {}).await
}

async fn run_ffmpeg(args: &[String], ctx: &JobCtx, duration_us: Option<f64>, sink: &dyn ProgressSink) -> EngineResult<()> {
    sink.report(duration_us.map(|_| 0.0), "Encoding");
    sidecar::run(ctx.tools.ffmpeg()?, args, RunOptions { cwd: None, cancel: &ctx.cancel }, |line| {
        // `-progress` emits key=value blocks. out_time_us is microseconds (as is the
        // misleadingly named out_time_ms — a long-standing ffmpeg quirk).
        if let (Some(total), Some(v)) = (duration_us, line.strip_prefix("out_time_us=")) {
            if let Ok(us) = v.trim().parse::<f64>() {
                sink.report(Some((us / total).clamp(0.0, 1.0) as f32), "Encoding");
            }
        }
    })
    .await
}

/// Container duration in µs, or None (live streams, images, broken headers →
/// the bar shows indeterminate instead of lying).
async fn probe_duration(spec: &ConvertSpec, ctx: &JobCtx) -> Option<f64> {
    let ffprobe = ctx.tools.ffprobe().ok()?;
    let a = [
        "-v".into(), "error".into(), "-show_entries".into(), "format=duration".into(),
        "-of".into(), "default=noprint_wrappers=1:nokey=1".into(), path_arg(&spec.input),
    ];
    let mut secs = None;
    sidecar::run(ffprobe, a, RunOptions { cwd: None, cancel: &ctx.cancel }, |l: &str| {
        secs = secs.or_else(|| l.trim().parse::<f64>().ok());
    })
    .await
    .ok()?;
    secs.filter(|s| *s > 0.0).map(|s| s * 1_000_000.0)
}

async fn frames_zip(spec: &ConvertSpec, ctx: &JobCtx, duration: Option<f64>) -> EngineResult<()> {
    let dir = spec.output.with_extension("omni-frames");
    fs::create_dir_all(&dir)?;
    let result = async {
        let extract = Scaled { inner: &*ctx.progress, start: 0.0, end: 0.85 };
        run_ffmpeg(&args::frames(spec, &dir), ctx, duration, &extract).await?;
        ctx.progress.report(Some(0.85), "Zipping frames");
        let (spec_out, dir2, cancel) = (spec.output.clone(), dir.clone(), ctx.cancel.clone());
        tokio::task::spawn_blocking(move || zip_dir(&dir2, &spec_out, &cancel))
            .await
            .unwrap_or_else(|e| Err(EngineError::Encode(e.to_string())))
    }
    .await;
    let _ = fs::remove_dir_all(&dir);
    result
}

fn zip_dir(dir: &Path, out: &Path, cancel: &tokio_util::sync::CancellationToken) -> EngineResult<()> {
    let mut entries: Vec<_> = fs::read_dir(dir)?.flatten().map(|e| e.path()).collect();
    entries.sort();
    let mut zip = ZipWriter::new(BufWriter::new(File::create(out)?));
    // PNG is already deflate-compressed; Stored avoids burning CPU for ~0% gain.
    let opts = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
    for p in entries {
        if cancel.is_cancelled() {
            return Err(EngineError::Cancelled);
        }
        let name = p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        zip.start_file(name, opts).map_err(EngineError::encode)?;
        io::copy(&mut File::open(&p)?, &mut zip)?;
    }
    zip.finish().map_err(EngineError::encode)?;
    Ok(())
}

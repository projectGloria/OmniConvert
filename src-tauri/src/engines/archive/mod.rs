//! Archive engine: repack between ZIP/7Z/TAR/TGZ/TXZ, decompress GZ/XZ, extract.

mod read;
mod write;

use std::fs;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use super::error::EngineResult;
use super::util::CancelReader;
use super::{ConvertSpec, JobCtx};
use write::Sink;

pub fn convert(spec: &ConvertSpec, ctx: &JobCtx) -> EngineResult<()> {
    let total = fs::metadata(&spec.input)?.len().max(1) as f64;
    let pos = Arc::new(AtomicU64::new(0));
    let mut sink = Sink::open(spec.target, &spec.output)?;

    let progress = &*ctx.progress;
    read::for_each(spec.src, &spec.input, pos.clone(), &mut |meta, data| {
        ctx.check()?;
        // Progress = share of the *compressed source* consumed: accurate even for tar streams.
        progress.report(Some((pos.load(Ordering::Relaxed) as f64 / total) as f32), "Repacking");
        sink.add(meta, &mut CancelReader::new(data, &ctx.cancel))
    })?;

    ctx.check()?;
    progress.report(Some(1.0), "Finalizing");
    sink.finish()
}

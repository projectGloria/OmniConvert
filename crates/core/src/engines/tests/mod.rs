//! Engine tests: real conversions, no Tauri runtime, no sidecars (PLAN D9).
//! Sidecar routes are covered by the Phase 5 `--ignored` e2e suite.

mod archive;
mod data;
mod image;
mod merge;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use tokio_util::sync::CancellationToken;

use super::{ConvertSpec, Ctx};
use crate::progress::NullSink;
use crate::registry::{Format, TargetKind};
use crate::settings::JobSettings;

pub fn ctx() -> Ctx {
    Ctx {
        cancel: CancellationToken::new(),
        progress: Arc::new(NullSink),
    }
}

pub fn spec(input: &Path, src: Format, target: TargetKind, output: PathBuf) -> ConvertSpec {
    ConvertSpec { input: input.to_path_buf(), src, target, output, settings: JobSettings::default() }
}

pub fn to(f: Format) -> TargetKind {
    TargetKind::Convert(f)
}

//! End-to-end route tests: real files in, real files out, through the same
//! detect → matrix → plan → engine → finalize path the queue uses (testkit::Harness).
//!
//!   cargo test --test e2e_routes                     native engines only
//!   npm run test:e2e   (adds -- --include-ignored)   + ffmpeg / pandoc / typst / pdfium
//!
//! Sidecar routes are `#[ignore]`d so a fresh clone without `npm run sidecars` stays green.
//! Laid out as `tests/e2e_routes/main.rs` so helpers can live in sibling modules without
//! Cargo compiling each of them as a separate (empty) test binary.

mod check;
mod fixtures;
mod native;
mod sidecar;

use std::path::{Path, PathBuf};

use omniconvert_lib::testkit::{Format, Harness, JobSettings, TargetKind};

/// Convert with default settings, panicking with the engine's message on failure.
pub fn run(h: &Harness, input: &Path, target: TargetKind) -> PathBuf {
    run_with(h, input, target, JobSettings::default())
}

pub fn run_with(h: &Harness, input: &Path, target: TargetKind, settings: JobSettings) -> PathBuf {
    h.convert(input, target, settings)
        .unwrap_or_else(|e| panic!("{} → {}: {e}", input.display(), target.id()))
}

pub fn to(f: Format) -> TargetKind {
    TargetKind::Convert(f)
}

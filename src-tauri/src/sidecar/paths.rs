//! Locates bundled sidecars and native libraries.

use std::path::{Path, PathBuf};

use serde::Serialize;
use ts_rs::TS;

use crate::engines::error::{EngineError, EngineResult};

const TRIPLE: &str = "x86_64-pc-windows-msvc";

/// Resolved paths of every external tool. Missing tools are `None`; engines turn that
/// into a clear "run fetch-sidecars.ps1" error instead of a cryptic spawn failure.
#[derive(Debug, Clone, Default)]
pub struct Tools {
    pub ffmpeg: Option<PathBuf>,
    pub ffprobe: Option<PathBuf>,
    pub pandoc: Option<PathBuf>,
    pub typst: Option<PathBuf>,
    pub pdfium: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct ToolStatus {
    pub ffmpeg: bool,
    pub ffprobe: bool,
    pub pandoc: bool,
    pub typst: bool,
    pub pdfium: bool,
}

impl Tools {
    pub fn discover(resource_dir: Option<&Path>) -> Self {
        Self {
            ffmpeg: locate_exe("ffmpeg"),
            ffprobe: locate_exe("ffprobe"),
            pandoc: locate_exe("pandoc"),
            typst: locate_exe("typst"),
            pdfium: locate_pdfium(resource_dir),
        }
    }

    pub fn status(&self) -> ToolStatus {
        ToolStatus {
            ffmpeg: self.ffmpeg.is_some(),
            ffprobe: self.ffprobe.is_some(),
            pandoc: self.pandoc.is_some(),
            typst: self.typst.is_some(),
            pdfium: self.pdfium.is_some(),
        }
    }

    pub fn ffmpeg(&self) -> EngineResult<&Path> {
        self.ffmpeg.as_deref().ok_or(EngineError::MissingTool("ffmpeg"))
    }
    pub fn ffprobe(&self) -> EngineResult<&Path> {
        self.ffprobe.as_deref().ok_or(EngineError::MissingTool("ffprobe"))
    }
    pub fn pandoc(&self) -> EngineResult<&Path> {
        self.pandoc.as_deref().ok_or(EngineError::MissingTool("pandoc"))
    }
    pub fn typst(&self) -> EngineResult<&Path> {
        self.typst.as_deref().ok_or(EngineError::MissingTool("typst"))
    }
    pub fn pdfium(&self) -> EngineResult<&Path> {
        self.pdfium.as_deref().ok_or(EngineError::MissingTool("pdfium.dll"))
    }
}

/// Installed app: Tauri strips the target triple and places `ffmpeg.exe` next to the
/// main exe. Dev/tests: fall back to `src-tauri/binaries/`. The `dev/` copies have
/// plain names, which matters for typst: pandoc decides how to drive a `--pdf-engine`
/// from its *basename*, and `typst-x86_64-pc-windows-msvc` isn't one it recognizes.
fn locate_exe(name: &str) -> Option<PathBuf> {
    let exe_dir = std::env::current_exe().ok()?.parent()?.to_path_buf();
    let mut candidates = vec![exe_dir.join(format!("{name}.exe"))];
    if cfg!(debug_assertions) {
        let bins = Path::new(env!("CARGO_MANIFEST_DIR")).join("binaries");
        candidates.push(bins.join("dev").join(format!("{name}.exe")));
        candidates.push(bins.join(format!("{name}-{TRIPLE}.exe")));
    }
    candidates.into_iter().find(|p| p.is_file())
}

fn locate_pdfium(resource_dir: Option<&Path>) -> Option<PathBuf> {
    let mut candidates: Vec<PathBuf> = resource_dir.map(|d| d.join("pdfium.dll")).into_iter().collect();
    if cfg!(debug_assertions) {
        candidates.push(Path::new(env!("CARGO_MANIFEST_DIR")).join("resources").join("pdfium.dll"));
    }
    candidates.into_iter().find(|p| p.is_file())
}

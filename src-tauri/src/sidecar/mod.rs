//! External processes (ffmpeg, ffprobe, pandoc, typst) — spawned from Rust only (PLAN D2).

mod job_object;
pub mod paths;
pub mod spawn;

pub use paths::{ToolStatus, Tools};
pub use spawn::{run, RunOptions};

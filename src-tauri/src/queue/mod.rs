//! Async job queue: Tokio tasks, per-pool semaphores, cancellation tokens, Tauri events.

pub mod job;
pub mod journal;
mod merge;
pub use omniconvert_core::output;
pub mod progress;
mod runner;

pub use runner::JobQueue;

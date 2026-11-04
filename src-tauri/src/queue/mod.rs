//! Async job queue: Tokio tasks, per-pool semaphores, cancellation tokens, Tauri events.

pub mod job;
pub mod journal;
pub mod output;
pub mod progress;
mod runner;

pub use runner::JobQueue;

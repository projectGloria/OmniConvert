//! OmniConvert core: the format registry (single source of truth, PLAN D1) and every
//! engine that is pure Rust. Shared by the Tauri desktop shell and the WASI web engine.
//!
//! Nothing here knows about Tauri, Tokio's runtime, sidecars or GPUs (PLAN D9): engines
//! take a `ConvertSpec` + `Ctx` and read/write plain paths. On the web those paths live
//! in the worker's in-memory WASI filesystem, so the same code runs unchanged.

pub mod engines;
pub mod output;
pub mod progress;
pub mod registry;
pub mod settings;

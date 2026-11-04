//! Hardware encoder detection + quality presets.

mod detect;
pub mod presets;

pub use detect::{detect, GpuCaps};

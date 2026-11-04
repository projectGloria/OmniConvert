use tauri::State;

use crate::gpu::GpuCaps;
use crate::sidecar::ToolStatus;
use crate::state::AppState;

/// Current GPU probe result (`ready: false` while still probing; the final value is
/// also pushed as `system://gpu-ready`).
#[tauri::command]
pub fn gpu_caps(state: State<'_, AppState>) -> GpuCaps {
    state.gpu_snapshot()
}

/// Which sidecars are present — lets the UI explain a disabled route up front.
#[tauri::command]
pub fn tool_status(state: State<'_, AppState>) -> ToolStatus {
    state.tools.status()
}

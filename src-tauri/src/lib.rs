mod commands;
mod engines;
mod error;
mod gpu;
mod queue;
mod registry;
mod sidecar;
mod state;
#[doc(hidden)]
pub mod testkit;

use tauri::{Emitter, Manager, RunEvent};

use sidecar::Tools;
use state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let tools = Tools::discover(app.path().resource_dir().ok().as_deref());
            let state = AppState::new(tools, app.path().app_local_data_dir().ok().as_deref());
            let (tools, gpu) = (state.tools.clone(), state.gpu.clone());
            app.manage(state);

            // GPU probing spawns up to 6 ffmpeg test encodes — never block window creation on it.
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let caps = gpu::detect(&tools).await;
                if let Ok(mut g) = gpu.write() {
                    *g = caps.clone();
                }
                let _ = handle.emit("system://gpu-ready", caps);
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::probe::probe_files,
            commands::jobs::enqueue_jobs,
            commands::jobs::enqueue_merge,
            commands::jobs::cancel_job,
            commands::jobs::cancel_all,
            commands::system::gpu_caps,
            commands::system::tool_status,
        ])
        .build(tauri::generate_context!())
        .expect("failed to build OmniConvert");

    app.run(|handle, event| {
        // Graceful path: cancel everything so sidecars are terminated via their Job
        // Objects and part files are cleaned up. (Crash path: the OS closes the Job
        // Object handles and KILL_ON_JOB_CLOSE does the same.)
        if let RunEvent::Exit = event {
            if let Some(state) = handle.try_state::<AppState>() {
                state.queue.cancel_all();
            }
        }
    });
}

use tauri::{AppHandle, State};

use crate::queue::job::JobRequest;
use crate::state::AppState;

/// Fire-and-forget: returns immediately; outcomes arrive as `conversion://*` events.
/// Per-job validation failures are reported as that job's error event, so one bad
/// file never rejects the whole batch.
#[tauri::command]
pub fn enqueue_jobs(app: AppHandle, state: State<'_, AppState>, jobs: Vec<JobRequest>) {
    let gpu = state.gpu_snapshot();
    for job in jobs {
        state.queue.enqueue(app.clone(), job, state.tools.clone(), gpu.clone());
    }
}

#[tauri::command]
pub fn cancel_job(state: State<'_, AppState>, job_id: String) {
    state.queue.cancel(&job_id);
}

#[tauri::command]
pub fn cancel_all(state: State<'_, AppState>) {
    state.queue.cancel_all();
}

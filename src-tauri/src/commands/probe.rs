use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::error::{AppError, AppResult};

/// Hard cap so dropping `C:\` doesn't walk the whole disk and freeze the queue.
const MAX_FILES: usize = 5_000;

/// Phase 1 probe result. Phase 2 extends this with `format`, `category` and `targets`
/// from the registry (and moves it to ts-rs generated bindings).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProbedFile {
    path: String,
    name: String,
    ext: String,
    size: u64,
}

/// Stats dropped paths; directories are expanded recursively.
///
/// `async` matters: synchronous Tauri commands run on the main (UI) thread, so a
/// slow `metadata()` on a network share would freeze window dragging. The fs work
/// is pushed to the blocking pool so it doesn't stall Tauri's async workers either.
#[tauri::command]
pub async fn probe_files(paths: Vec<String>) -> AppResult<Vec<ProbedFile>> {
    tauri::async_runtime::spawn_blocking(move || {
        let mut out = Vec::new();
        for p in paths {
            collect(Path::new(&p), &mut out);
            if out.len() >= MAX_FILES {
                out.truncate(MAX_FILES);
                break;
            }
        }
        out
    })
    .await
    .map_err(|e| AppError::Task(e.to_string()))
}

/// Unreadable entries are skipped rather than failing the whole drop —
/// one locked file shouldn't reject the other 200.
fn collect(path: &Path, out: &mut Vec<ProbedFile>) {
    let Ok(meta) = fs::metadata(path) else { return };

    if meta.is_dir() {
        let Ok(entries) = fs::read_dir(path) else { return };
        let mut children: Vec<PathBuf> = entries.flatten().map(|e| e.path()).collect();
        children.sort(); // deterministic queue order, matches Explorer's name sort closely enough
        for child in children {
            if out.len() >= MAX_FILES {
                return;
            }
            collect(&child, out);
        }
        return;
    }

    if !meta.is_file() {
        return;
    }

    out.push(ProbedFile {
        path: path.to_string_lossy().into_owned(),
        name: path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default(),
        ext: path
            .extension()
            .map(|e| e.to_string_lossy().to_ascii_lowercase())
            .unwrap_or_default(),
        size: meta.len(),
    });
}

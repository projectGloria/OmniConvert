use std::fs;
use std::path::{Path, PathBuf};

use crate::error::{AppError, AppResult};
use crate::registry::{self, FileInfo};

/// Hard cap so dropping `C:\` doesn't walk the whole disk and freeze the queue.
const MAX_FILES: usize = 5_000;

/// Stats, detects and resolves conversion targets for dropped paths.
/// Directories are expanded recursively.
///
/// `async` matters: synchronous Tauri commands run on the main (UI) thread, so a
/// slow `metadata()` on a network share would freeze window dragging. The fs work
/// is pushed to the blocking pool so it doesn't stall Tauri's async workers either.
#[tauri::command]
pub async fn probe_files(paths: Vec<String>) -> AppResult<Vec<FileInfo>> {
    tauri::async_runtime::spawn_blocking(move || {
        let mut files = Vec::new();
        for p in paths {
            collect(Path::new(&p), &mut files);
            if files.len() >= MAX_FILES {
                files.truncate(MAX_FILES);
                break;
            }
        }
        // Inspection (magic bytes, content sniffing) happens after the walk so the
        // cap applies to cheap stat calls, not to file reads.
        files.into_iter().map(|(path, size)| registry::inspect(&path, size)).collect()
    })
    .await
    .map_err(|e| AppError::Task(e.to_string()))
}

/// Unreadable entries are skipped rather than failing the whole drop —
/// one locked file shouldn't reject the other 200.
fn collect(path: &Path, out: &mut Vec<(PathBuf, u64)>) {
    let Ok(meta) = fs::metadata(path) else { return };

    if meta.is_dir() {
        let Ok(entries) = fs::read_dir(path) else { return };
        let mut children: Vec<PathBuf> = entries.flatten().map(|e| e.path()).collect();
        children.sort(); // deterministic queue order, close to Explorer's name sort
        for child in children {
            if out.len() >= MAX_FILES {
                return;
            }
            collect(&child, out);
        }
    } else if meta.is_file() {
        out.push((path.to_path_buf(), meta.len()));
    }
}

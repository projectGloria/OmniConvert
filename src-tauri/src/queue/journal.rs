//! Crash-safe cleanup of part outputs.
//!
//! On a clean exit or cancel, the runner deletes part files itself. A hard kill or
//! crash runs no code at all: the Job Object kills the sidecars, but the half-written
//! `*.omni-part.*` stays on disk. So every in-flight part path is journaled to the
//! app-data dir, and the next launch sweeps whatever a dead process left behind.

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

pub struct Journal {
    file: Option<PathBuf>,
    live: Mutex<HashSet<PathBuf>>,
}

impl Journal {
    /// `dir: None` (no app-data dir resolvable) degrades to a no-op journal.
    pub fn open(dir: Option<&Path>) -> Self {
        let file = dir.map(|d| d.join("inflight.journal"));
        if let Some(f) = &file {
            sweep(f);
        }
        Self { file, live: Mutex::default() }
    }

    pub fn add(&self, part: &Path) {
        self.update(|set| {
            set.insert(part.to_path_buf());
        });
    }

    pub fn remove(&self, part: &Path) {
        self.update(|set| {
            set.remove(part);
        });
    }

    fn update(&self, f: impl FnOnce(&mut HashSet<PathBuf>)) {
        let Some(file) = &self.file else { return };
        let Ok(mut set) = self.live.lock() else { return };
        f(&mut set);
        // Rewritten whole on each change: it holds at most a handful of lines.
        let body: String = set.iter().map(|p| format!("{}\n", p.display())).collect();
        if let Some(parent) = file.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let _ = fs::write(file, body);
    }
}

/// Delete parts listed by a previous (crashed) run. Only paths carrying our
/// `.omni-part` marker are touched — a corrupted journal can never delete user files.
fn sweep(journal: &Path) {
    let Ok(text) = fs::read_to_string(journal) else { return };
    for line in text.lines().map(str::trim).filter(|l| !l.is_empty()) {
        let p = Path::new(line);
        let is_ours = p.file_name().is_some_and(|n| n.to_string_lossy().contains(".omni-part"));
        if !is_ours {
            continue;
        }
        let _ = if p.is_dir() { fs::remove_dir_all(p) } else { fs::remove_file(p) };
    }
    let _ = fs::remove_file(journal);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn next_launch_sweeps_parts_but_never_foreign_paths() {
        let d = tempfile::tempdir().unwrap();
        let part = d.path().join("clip.omni-part.webm");
        let user_file = d.path().join("precious.docx");
        fs::write(&part, b"half").unwrap();
        fs::write(&user_file, b"keep").unwrap();

        let j = Journal::open(Some(d.path()));
        j.add(&part);
        j.add(&user_file); // simulate a corrupted/hostile journal entry
        drop(j); // "crash": no remove() ever runs

        Journal::open(Some(d.path()));
        assert!(!part.exists());
        assert!(user_file.exists());
    }
}

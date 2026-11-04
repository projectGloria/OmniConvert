//! Output path planning: naming, collision avoidance, temp ("part") paths, atomic finalize.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::registry::{Format, TargetKind};

#[derive(Debug, Clone)]
pub struct PlannedOutput {
    /// Where the finished output will live (file or directory).
    pub final_path: PathBuf,
    /// Where the engine writes. Keeps the real extension so tools like ffmpeg/pandoc
    /// can infer the container from it — but the `.omni-part` infix makes partial
    /// files obvious in Explorer and they're deleted on failure/cancel (PLAN D4).
    pub part_path: PathBuf,
    pub is_dir: bool,
}

/// Decide the output for converting `input` (detected as `src`) into `target`.
/// `out_dir: None` writes next to the source. `taken` reports paths reserved by
/// in-flight jobs that don't exist on disk yet.
pub fn plan(
    input: &Path,
    src: Format,
    target: TargetKind,
    out_dir: Option<&Path>,
    taken: impl Fn(&Path) -> bool,
) -> io::Result<PlannedOutput> {
    let dir = match out_dir {
        Some(d) => d.to_path_buf(),
        None => input.parent().map(Path::to_path_buf).unwrap_or_default(),
    };
    let stem = file_stem(input);

    let (suffix, ext, is_dir): (&str, Option<&str>, bool) = match target {
        TargetKind::Convert(f) if src == Format::Pdf && matches!(f, Format::Png | Format::Jpg) => ("_pages", None, true),
        TargetKind::Convert(f) => ("", Some(f.ext()), false),
        TargetKind::AnimatedGif => ("", Some("gif"), false),
        TargetKind::AnimatedWebp => ("", Some("webp"), false),
        TargetKind::FramesZip => ("_frames", Some("zip"), false),
        TargetKind::Extract => ("", None, true),
    };

    let base = format!("{stem}{suffix}");
    let final_path = unique_path(&dir, &base, ext, &taken);
    let final_name = final_path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let part_name = match ext {
        Some(e) => format!("{}.omni-part.{e}", final_name.trim_end_matches(&format!(".{e}"))),
        None => format!("{final_name}.omni-part"),
    };
    Ok(PlannedOutput { part_path: dir.join(part_name), final_path, is_dir })
}

/// File stem without compound archive suffixes: `backup.tar.gz` → `backup`.
fn file_stem(p: &Path) -> String {
    let name = p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "output".into());
    let lower = name.to_ascii_lowercase();
    for compound in [".tar.gz", ".tar.xz"] {
        if lower.ends_with(compound) {
            return name[..name.len() - compound.len()].to_owned();
        }
    }
    p.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or(name)
}

/// `photo.png` → `photo (1).png` → `photo (2).png` … Explorer-style, never overwrites.
fn unique_path(dir: &Path, base: &str, ext: Option<&str>, taken: &impl Fn(&Path) -> bool) -> PathBuf {
    let make = |n: u32| {
        let name = if n == 0 { base.to_owned() } else { format!("{base} ({n})") };
        dir.join(match ext {
            Some(e) => format!("{name}.{e}"),
            None => name,
        })
    };
    (0..10_000).map(make).find(|p| !p.exists() && !taken(p)).unwrap_or_else(|| make(10_000))
}

/// Atomically promote the part path to the final path. Same directory ⇒ same
/// volume ⇒ `rename` is a metadata op, never a copy.
pub fn finalize(out: &PlannedOutput) -> io::Result<PathBuf> {
    fs::rename(&out.part_path, &out.final_path)?;
    Ok(out.final_path.clone())
}

/// Best-effort removal of a failed/cancelled part output.
pub fn discard(out: &PlannedOutput) {
    let _ = if out.is_dir { fs::remove_dir_all(&out.part_path) } else { fs::remove_file(&out.part_path) };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plans_names_and_avoids_collisions() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("clip.mp4");
        fs::write(&input, b"x").unwrap();

        let p = plan(&input, Format::Mp4, TargetKind::Convert(Format::Mp3), None, |_| false).unwrap();
        assert_eq!(p.final_path, dir.path().join("clip.mp3"));
        assert_eq!(p.part_path, dir.path().join("clip.omni-part.mp3"));

        fs::write(dir.path().join("clip.mp3"), b"taken").unwrap();
        let p = plan(&input, Format::Mp4, TargetKind::Convert(Format::Mp3), None, |_| false).unwrap();
        assert_eq!(p.final_path, dir.path().join("clip (1).mp3"));

        let tgz = dir.path().join("backup.tar.gz");
        let p = plan(&tgz, Format::Tgz, TargetKind::Extract, None, |_| false).unwrap();
        assert_eq!(p.final_path, dir.path().join("backup"));
        assert!(p.is_dir);
    }
}

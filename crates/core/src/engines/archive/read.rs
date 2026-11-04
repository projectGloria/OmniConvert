//! Streaming entry iteration over every supported archive format. Nothing is
//! extracted to disk first: each entry's bytes flow straight into the sink.

use std::fs::File;
use std::io::{BufReader, Read};
use std::path::{Component, Path};
use std::sync::atomic::AtomicU64;
use std::sync::Arc;

use flate2::read::GzDecoder;
use sevenz_rust2::{ArchiveReader, Password};
use lzma_rust2::XzReader;

use crate::engines::error::{EngineError, EngineResult};
use crate::engines::util::Tracked;
use crate::registry::Format;

pub struct EntryMeta {
    /// Sanitized relative path with `/` separators.
    pub path: String,
    pub is_dir: bool,
    pub size: u64,
}

pub type Visit<'a> = dyn FnMut(&EntryMeta, &mut dyn Read) -> EngineResult<()> + 'a;

/// Calls `visit` for every safe entry. `pos` tracks bytes consumed from the source file.
pub fn for_each(src: Format, path: &Path, pos: Arc<AtomicU64>, visit: &mut Visit<'_>) -> EngineResult<()> {
    let file = Tracked::new(BufReader::new(File::open(path)?), pos);
    match src {
        Format::Zip => zip_entries(file, visit),
        Format::Tar => tar_entries(file, visit),
        Format::Tgz => tar_entries(GzDecoder::new(file), visit),
        Format::Txz => tar_entries(XzReader::new(file, true), visit),
        Format::SevenZ => sevenz_entries(file, visit),
        // Single-stream compressors: one entry named after the file minus its suffix.
        Format::Gz | Format::Xz => {
            let name = path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| "data".into());
            let meta = EntryMeta { path: name, is_dir: false, size: 0 };
            if src == Format::Gz {
                visit(&meta, &mut GzDecoder::new(file))
            } else {
                visit(&meta, &mut XzReader::new(file, true))
            }
        }
        other => Err(EngineError::Unsupported(format!("{} is not an archive", other.label()))),
    }
}

fn zip_entries<R: Read + std::io::Seek>(r: R, visit: &mut Visit<'_>) -> EngineResult<()> {
    let mut archive = zip::ZipArchive::new(r).map_err(EngineError::decode)?;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(EngineError::decode)?;
        // enclosed_name() is zip's own zip-slip guard (rejects `..` and absolute paths).
        let Some(rel) = entry.enclosed_name().and_then(|p| sanitize(&p)) else { continue };
        let meta = EntryMeta { path: rel, is_dir: entry.is_dir(), size: entry.size() };
        visit(&meta, &mut entry)?;
    }
    Ok(())
}

fn tar_entries<R: Read>(r: R, visit: &mut Visit<'_>) -> EngineResult<()> {
    let mut archive = tar::Archive::new(r);
    for entry in archive.entries().map_err(EngineError::decode)? {
        let mut entry = entry.map_err(EngineError::decode)?;
        let kind = entry.header().entry_type();
        // Symlinks/hardlinks/devices are skipped: following them is how tar extraction escapes the target dir.
        if !(kind.is_file() || kind.is_dir()) {
            continue;
        }
        let Some(rel) = entry.path().ok().and_then(|p| sanitize(&p)) else { continue };
        let meta = EntryMeta { path: rel, is_dir: kind.is_dir(), size: entry.size() };
        visit(&meta, &mut entry)?;
    }
    Ok(())
}

fn sevenz_entries<R: Read + std::io::Seek>(r: R, visit: &mut Visit<'_>) -> EngineResult<()> {
    let mut archive = ArchiveReader::new(r, Password::empty()).map_err(EngineError::decode)?;
    // The 7z callback must return the crate's own error type, so ours is parked here
    // and the iteration stopped with Ok(false).
    let mut failure: Option<EngineError> = None;
    archive
        .for_each_entries(|entry, reader| {
            let Some(rel) = sanitize(Path::new(entry.name())) else { return Ok(true) };
            let meta = EntryMeta { path: rel, is_dir: entry.is_directory(), size: entry.size() };
            match visit(&meta, reader) {
                Ok(()) => Ok(true),
                Err(e) => {
                    failure = Some(e);
                    Ok(false)
                }
            }
        })
        .map_err(EngineError::decode)?;
    failure.map_or(Ok(()), Err)
}

/// Zip-slip defense: only plain relative components survive. Returns None for
/// absolute paths, drive prefixes, `..`, or empty names.
pub fn sanitize(p: &Path) -> Option<String> {
    let mut parts = Vec::new();
    for c in p.components() {
        match c {
            Component::Normal(s) => parts.push(s.to_string_lossy().into_owned()),
            Component::CurDir => {}
            _ => return None,
        }
    }
    (!parts.is_empty()).then(|| parts.join("/"))
}

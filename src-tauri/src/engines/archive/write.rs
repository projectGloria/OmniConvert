use std::fs::{self, File};
use std::io::{self, BufWriter, Read, Write};
use std::path::{Path, PathBuf};

use flate2::write::GzEncoder;
use flate2::Compression;
use sevenz_rust2::{ArchiveEntry, ArchiveWriter};
use xz2::write::XzEncoder;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

use super::read::EntryMeta;
use crate::engines::error::{EngineError, EngineResult};
use crate::registry::{Format, TargetKind};

/// Destination for entries. Each variant streams; nothing is buffered whole in memory.
pub enum Sink {
    Zip(Box<ZipWriter<BufWriter<File>>>), // boxed: ZipWriter is ~4.4 KB inline
    Tar(tar::Builder<TarOut>),
    SevenZ(ArchiveWriter<File>),
    Dir(PathBuf),
}

pub enum TarOut {
    Plain(BufWriter<File>),
    Gz(GzEncoder<BufWriter<File>>),
    Xz(XzEncoder<BufWriter<File>>),
}

impl Write for TarOut {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self {
            TarOut::Plain(w) => w.write(buf),
            TarOut::Gz(w) => w.write(buf),
            TarOut::Xz(w) => w.write(buf),
        }
    }
    fn flush(&mut self) -> io::Result<()> {
        match self {
            TarOut::Plain(w) => w.flush(),
            TarOut::Gz(w) => w.flush(),
            TarOut::Xz(w) => w.flush(),
        }
    }
}

impl Sink {
    pub fn open(target: TargetKind, out: &Path) -> EngineResult<Self> {
        let file = || -> io::Result<BufWriter<File>> { Ok(BufWriter::new(File::create(out)?)) };
        Ok(match target {
            TargetKind::Extract => {
                fs::create_dir_all(out)?;
                Sink::Dir(out.to_path_buf())
            }
            TargetKind::Convert(Format::Zip) => Sink::Zip(Box::new(ZipWriter::new(file()?))),
            TargetKind::Convert(Format::Tar) => Sink::Tar(tar::Builder::new(TarOut::Plain(file()?))),
            TargetKind::Convert(Format::Tgz) => {
                Sink::Tar(tar::Builder::new(TarOut::Gz(GzEncoder::new(file()?, Compression::default()))))
            }
            // xz preset 6 = the `xz` CLI default; 9 costs ~10× the time for a few % gain.
            TargetKind::Convert(Format::Txz) => Sink::Tar(tar::Builder::new(TarOut::Xz(XzEncoder::new(file()?, 6)))),
            TargetKind::Convert(Format::SevenZ) => Sink::SevenZ(ArchiveWriter::create(out).map_err(EngineError::encode)?),
            other => return Err(EngineError::Unsupported(format!("{} is not an archive target", other.id()))),
        })
    }

    pub fn add(&mut self, meta: &EntryMeta, data: &mut dyn Read) -> EngineResult<()> {
        match self {
            Sink::Zip(z) => {
                let opts = SimpleFileOptions::default()
                    .compression_method(CompressionMethod::Deflated)
                    .large_file(meta.size >= u64::from(u32::MAX)); // ZIP64 only when needed
                if meta.is_dir {
                    z.add_directory(format!("{}/", meta.path), opts).map_err(EngineError::encode)?;
                } else {
                    z.start_file(meta.path.as_str(), opts).map_err(EngineError::encode)?;
                    io::copy(data, z)?;
                }
            }
            Sink::Tar(t) => {
                let mut h = tar::Header::new_gnu();
                h.set_size(if meta.is_dir { 0 } else { meta.size });
                h.set_mode(if meta.is_dir { 0o755 } else { 0o644 });
                h.set_entry_type(if meta.is_dir { tar::EntryType::Directory } else { tar::EntryType::Regular });
                h.set_mtime(0);
                t.append_data(&mut h, &meta.path, data)?;
            }
            Sink::SevenZ(s) => {
                if meta.is_dir {
                    s.push_archive_entry::<&[u8]>(ArchiveEntry::new_directory(&meta.path), None).map_err(EngineError::encode)?;
                } else {
                    s.push_archive_entry(ArchiveEntry::new_file(&meta.path), Some(data)).map_err(EngineError::encode)?;
                }
            }
            Sink::Dir(root) => {
                // `meta.path` is already sanitized (no `..`, no absolute), so join can't escape `root`.
                let dest = root.join(&meta.path);
                if meta.is_dir {
                    fs::create_dir_all(&dest)?;
                } else {
                    if let Some(parent) = dest.parent() {
                        fs::create_dir_all(parent)?;
                    }
                    io::copy(data, &mut BufWriter::new(File::create(&dest)?))?;
                }
            }
        }
        Ok(())
    }

    /// Writes central directories / trailers. Skipping this leaves a corrupt archive.
    pub fn finish(self) -> EngineResult<()> {
        match self {
            Sink::Zip(z) => (*z).finish().map_err(EngineError::encode)?.flush()?,
            Sink::Tar(t) => match t.into_inner()? {
                TarOut::Plain(mut w) => w.flush()?,
                TarOut::Gz(g) => g.finish()?.flush()?,
                TarOut::Xz(x) => x.finish()?.flush()?,
            },
            Sink::SevenZ(s) => drop(s.finish()?),
            Sink::Dir(_) => {}
        }
        Ok(())
    }
}

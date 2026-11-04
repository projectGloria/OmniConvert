//! Small I/O helpers shared by the pure-Rust engines.

use std::io::{self, Read, Seek, SeekFrom};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use tokio_util::sync::CancellationToken;

/// Read wrapper that fails fast once the job is cancelled. Lets cancellation reach
/// *inside* library calls we don't control (tar's `append_data`, `io::copy`, 7z
/// encoders) — the next `read()` returns an error and the library unwinds.
pub struct CancelReader<'a, R> {
    inner: R,
    cancel: &'a CancellationToken,
}

impl<'a, R> CancelReader<'a, R> {
    pub fn new(inner: R, cancel: &'a CancellationToken) -> Self {
        Self { inner, cancel }
    }
}

impl<R: Read> Read for CancelReader<'_, R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.cancel.is_cancelled() {
            // Not ErrorKind::Interrupted — io::copy silently *retries* on that.
            return Err(io::Error::other("cancelled"));
        }
        self.inner.read(buf)
    }
}

/// Tracks the read position of the *source* file, giving byte-accurate progress
/// for any archive format, even streaming ones (tar.gz) with no up-front entry count.
pub struct Tracked<R> {
    inner: R,
    pos: Arc<AtomicU64>,
}

impl<R> Tracked<R> {
    pub fn new(inner: R, pos: Arc<AtomicU64>) -> Self {
        Self { inner, pos }
    }
}

impl<R: Read> Read for Tracked<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let n = self.inner.read(buf)?;
        self.pos.fetch_add(n as u64, Ordering::Relaxed);
        Ok(n)
    }
}

impl<R: Seek> Seek for Tracked<R> {
    fn seek(&mut self, to: SeekFrom) -> io::Result<u64> {
        let p = self.inner.seek(to)?;
        self.pos.store(p, Ordering::Relaxed);
        Ok(p)
    }
}

/// `\\?\C:\…` verbatim paths (what canonicalize returns) confuse ffmpeg's protocol
/// parsing and some pandoc readers; hand tools the plain form.
pub fn path_arg(p: &std::path::Path) -> String {
    dunce::simplified(p).to_string_lossy().into_owned()
}

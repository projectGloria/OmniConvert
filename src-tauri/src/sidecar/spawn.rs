//! Runs a sidecar with: no console window, a per-run Job Object, line-streamed stdout,
//! a bounded stderr tail for error messages, and cooperative cancellation.

use std::collections::VecDeque;
use std::ffi::OsStr;
use std::path::Path;
use std::process::Stdio;

use tokio::io::{AsyncBufReadExt, AsyncRead, BufReader};
use tokio::process::Command;
use tokio_util::sync::CancellationToken;

use super::job_object::JobObject;
use crate::engines::error::{EngineError, EngineResult};

/// CREATE_NO_WINDOW: without it every ffmpeg run flashes a console window, because a
/// GUI-subsystem parent has no console for the child to inherit.
const CREATE_NO_WINDOW: u32 = 0x0800_0000;
const STDERR_TAIL_LINES: usize = 12;

pub struct RunOptions<'a> {
    pub cwd: Option<&'a Path>,
    pub cancel: &'a CancellationToken,
}

/// Spawn `program args…`, call `on_line` for each stdout line, and wait.
/// Cancellation kills the whole process tree via the Job Object.
pub async fn run<I, S>(program: &Path, args: I, opts: RunOptions<'_>, mut on_line: impl FnMut(&str)) -> EngineResult<()>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let tool = program.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    // Normalize the triple-suffixed dev name back to the tool's real name for messages.
    let tool = tool.split('-').next().unwrap_or(&tool).to_owned();

    let mut cmd = Command::new(program);
    cmd.args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true) // belt and braces: dropping the future also kills the root process
        .creation_flags(CREATE_NO_WINDOW);
    if let Some(dir) = opts.cwd {
        cmd.current_dir(dir);
    }

    let mut child = cmd.spawn()?;
    let job = JobObject::new()?;
    if let Some(handle) = child.raw_handle() {
        job.assign(handle)?;
    }

    let stdout = child.stdout.take().expect("stdout piped");
    let stderr = child.stderr.take().expect("stderr piped");
    // stderr must be drained concurrently: if its pipe buffer fills, the child blocks
    // forever on write and we deadlock waiting for stdout EOF.
    let stderr_task = tokio::spawn(tail_lines(stderr));

    let mut lines = BufReader::new(stdout).lines();
    loop {
        tokio::select! {
            biased; // check cancel first so a chatty stdout can't starve it
            () = opts.cancel.cancelled() => {
                job.terminate();
                let _ = child.wait().await; // reap, so no zombie handle lingers
                return Err(EngineError::Cancelled);
            }
            line = lines.next_line() => match line {
                Ok(Some(l)) => on_line(&l),
                _ => break, // EOF or broken pipe: process is exiting
            },
        }
    }

    let status = tokio::select! {
        biased;
        () = opts.cancel.cancelled() => {
            job.terminate();
            let _ = child.wait().await;
            return Err(EngineError::Cancelled);
        }
        s = child.wait() => s?,
    };
    let tail = stderr_task.await.unwrap_or_default();

    if status.success() {
        Ok(())
    } else {
        Err(EngineError::Tool { tool, code: status.code().unwrap_or(-1), tail })
    }
}

async fn tail_lines(stream: impl AsyncRead + Unpin) -> String {
    let mut ring = VecDeque::with_capacity(STDERR_TAIL_LINES);
    let mut lines = BufReader::new(stream).lines();
    while let Ok(Some(line)) = lines.next_line().await {
        if line.trim().is_empty() {
            continue;
        }
        if ring.len() == STDERR_TAIL_LINES {
            ring.pop_front();
        }
        ring.push_back(line);
    }
    let joined = Vec::from(ring).join("\n");
    // Keep the end: that's where ffmpeg/pandoc put the actual reason.
    let keep = joined.char_indices().rev().nth(600).map_or(0, |(i, _)| i);
    joined[keep..].to_owned()
}

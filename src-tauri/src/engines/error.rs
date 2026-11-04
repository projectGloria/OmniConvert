use std::io;

/// Failure modes of a single conversion. Messages are user-facing (shown in the row).
#[derive(Debug, thiserror::Error)]
pub enum EngineError {
    #[error("cancelled")]
    Cancelled,

    #[error("{0}")]
    Io(#[from] io::Error),

    /// The input is valid but this conversion can't represent it (nested JSON → CSV, …).
    #[error("{0}")]
    Unsupported(String),

    /// The input couldn't be parsed/decoded.
    #[error("couldn't read input: {0}")]
    Decode(String),

    #[error("couldn't write output: {0}")]
    Encode(String),

    /// A sidecar is missing from the install (or `fetch-sidecars.ps1` wasn't run in dev).
    #[error("{0} is not available — run scripts/fetch-sidecars.ps1")]
    MissingTool(&'static str),

    /// Sidecar exited non-zero. `tail` is the last stderr lines, which carry the real reason.
    #[error("{tool} failed (exit {code}): {tail}")]
    Tool { tool: String, code: i32, tail: String },
}

impl EngineError {
    pub fn decode(e: impl std::fmt::Display) -> Self {
        Self::Decode(e.to_string())
    }

    pub fn encode(e: impl std::fmt::Display) -> Self {
        Self::Encode(e.to_string())
    }

    /// (row message, expandable detail). Tool failures keep the stderr tail out of the
    /// one-line row text — a 600-char ffmpeg dump truncated to "ffmpeg failed (exit 1): [h264 @ 0x…" helps nobody.
    pub fn into_parts(self) -> (String, Option<String>) {
        match self {
            Self::Tool { tool, code, tail } if !tail.is_empty() => (format!("{tool} failed (exit {code})"), Some(tail)),
            other => (other.to_string(), None),
        }
    }
}

pub type EngineResult<T> = Result<T, EngineError>;

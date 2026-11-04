use serde::{Serialize, Serializer};

/// App-wide error returned from commands. Tauri requires command errors to be
/// `Serialize`; we flatten to the Display string so the frontend gets a readable
/// message instead of an opaque `{}` (std::io::Error isn't Serialize).
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("background task failed: {0}")]
    Task(String),
}

impl Serialize for AppError {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}

pub type AppResult<T> = Result<T, AppError>;

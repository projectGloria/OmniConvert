use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::formats::{Category, Format};

/// What the user asked for. Most targets are a plain format switch; the rest are
/// operations whose output format alone doesn't describe them (a GIF from a video is
/// an *animation*, a ZIP from a video is *frame extraction*).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(tag = "type", content = "format", rename_all = "camelCase")]
#[ts(export)]
pub enum TargetKind {
    Convert(Format),
    AnimatedGif,
    AnimatedWebp,
    FramesZip,
    Extract,
}

impl TargetKind {
    /// Stable string key used by the UI for selection, React keys and "Apply to all".
    pub fn id(self) -> String {
        match self {
            TargetKind::Convert(f) => f.id().to_owned(),
            TargetKind::AnimatedGif => "anim-gif".into(),
            TargetKind::AnimatedWebp => "anim-webp".into(),
            TargetKind::FramesZip => "frames-zip".into(),
            TargetKind::Extract => "extract".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Target {
    pub id: String,
    pub kind: TargetKind,
    pub label: String,
    /// Picker section this target is listed under (usually the *output* category).
    pub group: Category,
    /// Short caveat shown under the label, e.g. "Layout may differ".
    pub note: Option<String>,
}

impl Target {
    pub fn new(kind: TargetKind, label: impl Into<String>, group: Category) -> Self {
        Self { id: kind.id(), kind, label: label.into(), group, note: None }
    }

    pub fn convert(f: Format) -> Self {
        Self::new(TargetKind::Convert(f), f.label(), f.category())
    }

    pub fn with_note(mut self, note: &str) -> Self {
        self.note = Some(note.to_owned());
        self
    }
}

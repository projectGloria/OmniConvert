use std::path::Path;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum Category {
    Video,
    Audio,
    Image,
    Document,
    Data,
    Archive,
}

/// Every format the app can read or write. The serde names are the wire IDs shared
/// with TS; `Format::id()` must stay in lockstep (enforced by a unit test).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
#[rustfmt::skip] // dense one-line rows are the readable form for a 46-variant table
pub enum Format {
    Mp4, Mkv, Mov, Avi, Webm, Flv,
    Mp3, Wav, Flac, Aac, Ogg, M4a, Opus,
    Png, Jpg, Webp, Avif, Ico, Bmp, Tiff, Gif, Svg,
    Pdf, Docx, Md, Html, Txt, Rtf, Epub,
    Xlsx, Xls, Ods, Csv, Tsv, Json, Yaml, Toml, Xml,
    /// Output-only: a GFM pipe table written to `.md`.
    #[serde(rename = "mdtable")]
    MdTable,
    Zip,
    #[serde(rename = "7z")]
    SevenZ,
    Tar, Gz, Tgz, Xz, Txz,
}

use Format::*;

/// Output groups, in the order the picker shows them.
pub const VIDEO_OUT: &[Format] = &[Mp4, Mkv, Mov, Avi, Webm, Flv];
pub const AUDIO_OUT: &[Format] = &[Mp3, Wav, Flac, Aac, Ogg, M4a, Opus];
pub const IMAGE_OUT: &[Format] = &[Png, Jpg, Webp, Avif, Ico, Bmp, Tiff, Gif];
pub const DOC_OUT: &[Format] = &[Docx, Pdf, Md, Html, Txt, Rtf, Epub];
pub const TABULAR_OUT: &[Format] = &[Csv, Tsv, Xlsx, MdTable];
pub const TREE_OUT: &[Format] = &[Json, Yaml, Toml, Xml];
pub const ARCHIVE_OUT: &[Format] = &[Zip, SevenZ, Tar, Tgz, Txz];

impl Format {
    pub fn category(self) -> Category {
        match self {
            Mp4 | Mkv | Mov | Avi | Webm | Flv => Category::Video,
            Mp3 | Wav | Flac | Aac | Ogg | M4a | Opus => Category::Audio,
            Png | Jpg | Webp | Avif | Ico | Bmp | Tiff | Gif | Svg => Category::Image,
            Pdf | Docx | Md | Html | Txt | Rtf | Epub => Category::Document,
            Xlsx | Xls | Ods | Csv | Tsv | Json | Yaml | Toml | Xml | MdTable => Category::Data,
            Zip | SevenZ | Tar | Gz | Tgz | Xz | Txz => Category::Archive,
        }
    }

    /// Stable wire ID — identical to the serde name.
    pub fn id(self) -> &'static str {
        match self {
            MdTable => "mdtable",
            SevenZ => "7z",
            // ext() is "tar.gz"/"tar.xz" for these — the wire id must not inherit that.
            Tgz => "tgz",
            Txz => "txz",
            other => other.ext(),
        }
    }

    /// Extension written for outputs (no leading dot).
    #[rustfmt::skip]
    pub fn ext(self) -> &'static str {
        match self {
            Mp4 => "mp4", Mkv => "mkv", Mov => "mov", Avi => "avi", Webm => "webm", Flv => "flv",
            Mp3 => "mp3", Wav => "wav", Flac => "flac", Aac => "aac", Ogg => "ogg", M4a => "m4a", Opus => "opus",
            Png => "png", Jpg => "jpg", Webp => "webp", Avif => "avif", Ico => "ico", Bmp => "bmp",
            Tiff => "tiff", Gif => "gif", Svg => "svg",
            Pdf => "pdf", Docx => "docx", Md | MdTable => "md", Html => "html", Txt => "txt", Rtf => "rtf", Epub => "epub",
            Xlsx => "xlsx", Xls => "xls", Ods => "ods", Csv => "csv", Tsv => "tsv",
            Json => "json", Yaml => "yaml", Toml => "toml", Xml => "xml",
            Zip => "zip", SevenZ => "7z", Tar => "tar", Gz => "gz", Tgz => "tar.gz", Xz => "xz", Txz => "tar.xz",
        }
    }

    /// Exhaustive match (no lookup table): adding a variant without a label is a compile error.
    #[rustfmt::skip]
    pub fn label(self) -> &'static str {
        match self {
            Mp4 => "MP4", Mkv => "MKV", Mov => "MOV", Avi => "AVI", Webm => "WEBM", Flv => "FLV",
            Mp3 => "MP3", Wav => "WAV", Flac => "FLAC", Aac => "AAC", Ogg => "OGG", M4a => "M4A", Opus => "OPUS",
            Png => "PNG", Jpg => "JPG", Webp => "WEBP", Avif => "AVIF", Ico => "ICO", Bmp => "BMP",
            Tiff => "TIFF", Gif => "GIF", Svg => "SVG",
            Pdf => "PDF", Docx => "DOCX", Md => "Markdown", Html => "HTML", Txt => "TXT", Rtf => "RTF", Epub => "EPUB",
            Xlsx => "XLSX", Xls => "XLS", Ods => "ODS", Csv => "CSV", Tsv => "TSV",
            Json => "JSON", Yaml => "YAML", Toml => "TOML", Xml => "XML", MdTable => "Markdown table",
            Zip => "ZIP", SevenZ => "7Z", Tar => "TAR", Gz => "GZ", Tgz => "TAR.GZ", Xz => "XZ", Txz => "TAR.XZ",
        }
    }

    /// Extension-based lookup. Handles double extensions (`.tar.gz`) and common aliases.
    pub fn from_path(path: &Path) -> Option<Format> {
        let name = path.file_name()?.to_string_lossy().to_ascii_lowercase();
        if name.ends_with(".tar.gz") {
            return Some(Tgz);
        }
        if name.ends_with(".tar.xz") {
            return Some(Txz);
        }
        Self::from_ext(path.extension()?.to_str()?)
    }

    #[rustfmt::skip]
    pub fn from_ext(ext: &str) -> Option<Format> {
        Some(match ext.to_ascii_lowercase().as_str() {
            "mp4" | "m4v" => Mp4, "mkv" => Mkv, "mov" => Mov, "avi" => Avi, "webm" => Webm, "flv" => Flv,
            "mp3" => Mp3, "wav" => Wav, "flac" => Flac, "aac" => Aac, "ogg" | "oga" => Ogg,
            "m4a" => M4a, "opus" => Opus,
            "png" => Png, "jpg" | "jpeg" | "jfif" => Jpg, "webp" => Webp, "avif" => Avif, "ico" => Ico,
            "bmp" => Bmp, "tif" | "tiff" => Tiff, "gif" => Gif, "svg" => Svg,
            "pdf" => Pdf, "docx" => Docx, "md" | "markdown" => Md, "html" | "htm" => Html,
            "txt" => Txt, "rtf" => Rtf, "epub" => Epub,
            "xlsx" => Xlsx, "xls" => Xls, "ods" => Ods, "csv" => Csv, "tsv" => Tsv,
            "json" => Json, "yaml" | "yml" => Yaml, "toml" => Toml, "xml" => Xml,
            "zip" => Zip, "7z" => SevenZ, "tar" => Tar, "gz" => Gz, "tgz" => Tgz, "xz" => Xz, "txz" => Txz,
            _ => return None,
        })
    }
}


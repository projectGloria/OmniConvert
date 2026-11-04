//! Cheap content sniffing that changes which targets make sense for a file.
//! Everything here is bounded by size caps — probing must stay fast on a 500-file drop.

use std::fs;
use std::path::Path;

use serde_json::Value;

use super::formats::Format;

const TREE_PARSE_CAP: u64 = 32 * 1024 * 1024;
const TEXT_SCAN_CAP: u64 = 4 * 1024 * 1024;
const GIF_SCAN_CAP: u64 = 64 * 1024 * 1024;

/// Whether a tree document (JSON/YAML/TOML) can be flattened into rows × columns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tabular {
    Yes,
    No,
    /// Too big to parse at probe time, unparseable, or a format we don't sniff (XML).
    Unknown,
}

/// Content traits that refine the target list beyond the format itself.
#[derive(Debug, Clone, Copy)]
pub struct Traits {
    pub animated: bool,
    pub md_table: bool,
    pub tabular: Tabular,
}

impl Traits {
    pub fn sniff(path: &Path, format: Format, size: u64) -> Self {
        Self {
            animated: format == Format::Gif && size <= GIF_SCAN_CAP && gif_is_animated(path),
            md_table: format == Format::Md && size <= TEXT_SCAN_CAP && md_has_table(path),
            tabular: match format {
                Format::Json | Format::Yaml | Format::Toml if size <= TREE_PARSE_CAP => tree_tabular(path, format),
                _ => Tabular::Unknown,
            },
        }
    }
}

/// Every GIF frame is preceded by a Graphic Control Extension (`21 F9 04`).
/// Two or more ⇒ animated. A heuristic, but it never needs to decode a pixel.
fn gif_is_animated(path: &Path) -> bool {
    let Ok(bytes) = fs::read(path) else { return false };
    bytes.windows(3).filter(|w| *w == [0x21, 0xF9, 0x04]).take(2).count() >= 2
}

/// GFM table = a pipe row immediately followed by a delimiter row like `|---|:--:|`.
fn md_has_table(path: &Path) -> bool {
    let Ok(text) = fs::read_to_string(path) else { return false };
    let lines: Vec<&str> = text.lines().map(str::trim).collect();
    lines.windows(2).any(|w| w[0].contains('|') && is_delimiter_row(w[1]))
}

fn is_delimiter_row(line: &str) -> bool {
    line.contains('|')
        && line.contains('-')
        && line.chars().all(|c| matches!(c, '|' | '-' | ':' | ' ' | '\t'))
}

fn tree_tabular(path: &Path, format: Format) -> Tabular {
    let Ok(text) = fs::read_to_string(path) else { return Tabular::Unknown };
    let parsed: Option<Value> = match format {
        Format::Json => serde_json::from_str(&text).ok(),
        Format::Yaml => serde_yaml_ng::from_str(&text).ok(),
        Format::Toml => toml::from_str(&text).ok(),
        _ => None,
    };
    match parsed {
        Some(v) if rows_of(&v).is_some() => Tabular::Yes,
        Some(_) => Tabular::No,
        None => Tabular::Unknown, // malformed: let the converter report the real error later
    }
}

/// The record array, if `v` is `[ {..}, .. ]` or `{ "anyKey": [ {..}, .. ] }`.
/// The second shape covers TOML's `[[rows]]` (TOML can't have a top-level array)
/// and the common `{ "data": [...] }` API envelope. Records must be flat: no nested
/// objects or arrays, since those have no single-cell representation.
pub fn rows_of(v: &Value) -> Option<&Vec<Value>> {
    let arr = match v {
        Value::Array(a) => a,
        Value::Object(m) if m.len() == 1 => m.values().next()?.as_array()?,
        _ => return None,
    };
    let flat = !arr.is_empty()
        && arr.iter().all(|row| {
            row.as_object()
                .is_some_and(|o| o.values().all(|c| !c.is_array() && !c.is_object()))
        });
    flat.then_some(arr)
}

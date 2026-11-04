//! XML ⇄ JSON-ish tree. XML has no canonical data model, so the convention is:
//! attributes → `@name`, mixed text → `#text`, repeated sibling elements → arrays,
//! and the root element is unwrapped. `<rows><row>…</row><row>…</row></rows>` reads
//! as `{ "row": [ {...}, {...} ] }` — exactly the record shape `rows_of` accepts.

use std::fs::File;
use std::io::{BufReader, BufWriter, Write};
use std::path::Path;

use quick_xml::escape::escape;
use quick_xml::events::Event;
use quick_xml::Reader;
use serde_json::{Map, Value};

use super::model::{cell_text, infer_cell, Doc};
use crate::engines::error::{EngineError, EngineResult};

struct Frame {
    name: String,
    map: Map<String, Value>,
    text: String,
}

pub fn read(path: &Path) -> EngineResult<Value> {
    let mut reader = Reader::from_reader(BufReader::new(File::open(path)?));
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();
    let mut stack: Vec<Frame> = vec![Frame { name: String::new(), map: Map::new(), text: String::new() }];

    loop {
        match reader.read_event_into(&mut buf).map_err(EngineError::decode)? {
            Event::Start(e) => stack.push(open(&e)?),
            Event::Empty(e) => {
                let frame = open(&e)?;
                attach(stack.last_mut().expect("root frame"), frame);
            }
            Event::Text(t) => {
                let s = t.unescape().map_err(EngineError::decode)?;
                stack.last_mut().expect("root frame").text.push_str(&s);
            }
            Event::CData(c) => stack.last_mut().expect("root frame").text.push_str(&String::from_utf8_lossy(&c)),
            Event::End(_) => {
                let frame = stack.pop().ok_or_else(|| EngineError::Decode("unbalanced XML".into()))?;
                attach(stack.last_mut().ok_or_else(|| EngineError::Decode("unbalanced XML".into()))?, frame);
            }
            Event::Eof => break,
            _ => {} // declarations, comments, PIs, doctype
        }
        buf.clear();
    }

    // The synthetic frame holds exactly the document root → unwrap it.
    let doc = stack.pop().map(|f| f.map).unwrap_or_default();
    Ok(doc.into_iter().next().map(|(_, v)| v).unwrap_or(Value::Null))
}

fn open(e: &quick_xml::events::BytesStart<'_>) -> EngineResult<Frame> {
    let name = String::from_utf8_lossy(e.name().as_ref()).into_owned();
    let mut map = Map::new();
    for attr in e.attributes() {
        let attr = attr.map_err(EngineError::decode)?;
        let key = format!("@{}", String::from_utf8_lossy(attr.key.as_ref()));
        map.insert(key, infer_cell(&attr.unescape_value().map_err(EngineError::decode)?));
    }
    Ok(Frame { name, map, text: String::new() })
}

fn attach(parent: &mut Frame, child: Frame) {
    let value = if child.map.is_empty() {
        infer_cell(&child.text) // leaf element → typed scalar
    } else {
        let mut map = child.map;
        if !child.text.trim().is_empty() {
            map.insert("#text".into(), Value::String(child.text));
        }
        Value::Object(map)
    };
    match parent.map.get_mut(&child.name) {
        Some(Value::Array(arr)) => arr.push(value),
        Some(existing) => *existing = Value::Array(vec![existing.take(), value]),
        None => {
            parent.map.insert(child.name, value);
        }
    }
}

pub fn write(doc: Doc, out: &Path) -> EngineResult<()> {
    let mut w = BufWriter::new(File::create(out)?);
    writeln!(w, r#"<?xml version="1.0" encoding="UTF-8"?>"#)?;
    match doc {
        // Tables use the conventional record layout.
        Doc::Table(_) => element(&mut w, "rows", &Value::Object(Map::from_iter([("row".into(), doc.into_tree())])), 0)?,
        // A bare top-level array would emit sibling roots = invalid XML; nest it as <item>s.
        Doc::Tree(v @ Value::Array(_)) => element(&mut w, "root", &Value::Object(Map::from_iter([("item".into(), v)])), 0)?,
        Doc::Tree(v) => element(&mut w, "root", &v, 0)?,
    }
    w.flush()?;
    Ok(())
}

fn element(w: &mut impl Write, name: &str, v: &Value, depth: usize) -> EngineResult<()> {
    let pad = "  ".repeat(depth);
    let tag = xml_name(name);
    match v {
        Value::Array(items) => {
            for item in items {
                element(w, name, item, depth)?; // arrays = repeated siblings with the same tag
            }
        }
        Value::Object(map) => {
            let attrs: String = map
                .iter()
                .filter_map(|(k, v)| k.strip_prefix('@').map(|a| format!(r#" {}="{}""#, xml_name(a), escape(cell_text(v).as_str()))))
                .collect();
            write!(w, "{pad}<{tag}{attrs}>")?;
            if let Some(text) = map.get("#text") {
                write!(w, "{}", escape(cell_text(text).as_str()))?;
            }
            writeln!(w)?;
            for (k, child) in map.iter().filter(|(k, _)| !k.starts_with('@') && *k != "#text") {
                element(w, k, child, depth + 1)?;
            }
            writeln!(w, "{pad}</{tag}>")?;
        }
        scalar => writeln!(w, "{pad}<{tag}>{}</{tag}>", escape(cell_text(scalar).as_str()))?,
    }
    Ok(())
}

/// JSON keys can be anything; XML names can't start with a digit or contain spaces.
fn xml_name(key: &str) -> String {
    let mut s: String = key.chars().map(|c| if c.is_alphanumeric() || "_-.".contains(c) { c } else { '_' }).collect();
    if s.is_empty() || s.starts_with(|c: char| c.is_ascii_digit() || c == '-' || c == '.') {
        s.insert(0, '_');
    }
    s
}

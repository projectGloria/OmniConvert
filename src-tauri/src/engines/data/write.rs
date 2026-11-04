use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

use rust_xlsxwriter::{Format as XlsxFormat, Workbook};
use serde_json::{Map, Value};

use super::model::{cell_text, Doc, Table};
use super::xml;
use crate::engines::error::{EngineError, EngineResult};
use crate::registry::Format;

const XLSX_MAX_ROWS: usize = 1_048_576;
const XLSX_MAX_COLS: usize = 16_384;

pub fn write(doc: Doc, dst: Format, out: &Path) -> EngineResult<()> {
    match dst {
        Format::Csv => delimited(&doc.into_table()?, b',', out),
        Format::Tsv => delimited(&doc.into_table()?, b'\t', out),
        Format::Xlsx => xlsx(&doc.into_table()?, out),
        Format::MdTable => markdown(&doc.into_table()?, out),
        Format::Json => {
            let mut w = BufWriter::new(File::create(out)?);
            serde_json::to_writer_pretty(&mut w, &doc.into_tree()).map_err(EngineError::encode)?;
            w.flush()?;
            Ok(())
        }
        Format::Yaml => {
            let w = BufWriter::new(File::create(out)?);
            serde_yaml_ng::to_writer(w, &doc.into_tree()).map_err(EngineError::encode)
        }
        Format::Toml => {
            let text = toml::to_string_pretty(&toml_compatible(doc.into_tree())).map_err(EngineError::encode)?;
            std::fs::write(out, text)?;
            Ok(())
        }
        Format::Xml => xml::write(doc, out),
        other => Err(EngineError::Unsupported(format!("{} is not a data output", other.label()))),
    }
}

fn delimited(t: &Table, delimiter: u8, out: &Path) -> EngineResult<()> {
    let mut w = csv::WriterBuilder::new().delimiter(delimiter).flexible(true).from_path(out).map_err(EngineError::encode)?;
    w.write_record(&t.headers).map_err(EngineError::encode)?;
    for row in &t.rows {
        w.write_record(row.iter().map(cell_text)).map_err(EngineError::encode)?;
    }
    w.flush()?;
    Ok(())
}

fn xlsx(t: &Table, out: &Path) -> EngineResult<()> {
    if t.rows.len() + 1 > XLSX_MAX_ROWS || t.headers.len() > XLSX_MAX_COLS {
        return Err(EngineError::Unsupported("too large for Excel (1,048,576 rows × 16,384 columns max)".into()));
    }
    let mut wb = Workbook::new();
    let ws = wb.add_worksheet();
    let bold = XlsxFormat::new().set_bold();
    let e = EngineError::encode;

    for (c, h) in t.headers.iter().enumerate() {
        ws.write_string_with_format(0, c as u16, h, &bold).map_err(e)?;
    }
    for (r, row) in t.rows.iter().enumerate() {
        let r = (r + 1) as u32;
        for (c, cell) in row.iter().enumerate() {
            let c = c as u16;
            match cell {
                Value::Null => {}
                Value::Bool(b) => { ws.write_boolean(r, c, *b).map_err(e)?; }
                Value::Number(n) => { ws.write_number(r, c, n.as_f64().unwrap_or_default()).map_err(e)?; }
                other => { ws.write_string(r, c, cell_text(other)).map_err(e)?; }
            }
        }
    }
    ws.set_freeze_panes(1, 0).map_err(e)?; // header stays visible while scrolling
    ws.autofit();
    wb.save(out).map_err(e)
}

fn markdown(t: &Table, out: &Path) -> EngineResult<()> {
    // Pipes and newlines would break the row structure.
    let esc = |s: String| s.replace('|', "\\|").replace(['\r', '\n'], " ");
    let mut w = BufWriter::new(File::create(out)?);
    writeln!(w, "| {} |", t.headers.iter().cloned().map(esc).collect::<Vec<_>>().join(" | "))?;
    writeln!(w, "|{}|", vec!["---"; t.headers.len().max(1)].join("|"))?;
    for row in &t.rows {
        writeln!(w, "| {} |", row.iter().map(|c| esc(cell_text(c))).collect::<Vec<_>>().join(" | "))?;
    }
    w.flush()?;
    Ok(())
}

/// TOML has no null and no top-level array: drop nulls, wrap arrays as `[[rows]]`.
fn toml_compatible(v: Value) -> Value {
    fn strip(v: Value) -> Option<Value> {
        match v {
            Value::Null => None,
            Value::Array(a) => Some(Value::Array(a.into_iter().filter_map(strip).collect())),
            Value::Object(o) => Some(Value::Object(o.into_iter().filter_map(|(k, v)| strip(v).map(|v| (k, v))).collect())),
            other => Some(other),
        }
    }
    match strip(v) {
        Some(Value::Object(o)) => Value::Object(o),
        Some(other) => Value::Object(Map::from_iter([("rows".to_owned(), other)])),
        None => Value::Object(Map::new()),
    }
}

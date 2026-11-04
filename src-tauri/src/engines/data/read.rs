use std::fs::{self, File};
use std::io::BufReader;
use std::path::Path;

use calamine::{open_workbook_auto, Data, DataType, Reader};
use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
use serde_json::{Number, Value};

use super::model::{infer_cell, Doc, Table};
use super::xml;
use crate::engines::error::{EngineError, EngineResult};
use crate::registry::Format;

pub fn read(path: &Path, src: Format) -> EngineResult<Doc> {
    Ok(match src {
        Format::Csv => Doc::Table(delimited(path, b',')?),
        Format::Tsv => Doc::Table(delimited(path, b'\t')?),
        Format::Xlsx | Format::Xls | Format::Ods => Doc::Table(spreadsheet(path)?),
        Format::Md => Doc::Table(markdown_table(&fs::read_to_string(path)?)?),
        Format::Json => Doc::Tree(serde_json::from_reader(BufReader::new(File::open(path)?)).map_err(EngineError::decode)?),
        Format::Yaml => Doc::Tree(serde_yaml_ng::from_reader(BufReader::new(File::open(path)?)).map_err(EngineError::decode)?),
        Format::Toml => Doc::Tree(toml::from_str(&fs::read_to_string(path)?).map_err(EngineError::decode)?),
        Format::Xml => Doc::Tree(xml::read(path)?),
        other => return Err(EngineError::Unsupported(format!("{} is not a data input", other.label()))),
    })
}

fn delimited(path: &Path, delimiter: u8) -> EngineResult<Table> {
    let mut rdr = csv::ReaderBuilder::new()
        .delimiter(delimiter)
        .flexible(true) // tolerate ragged rows instead of failing the whole file
        .from_path(path)
        .map_err(EngineError::decode)?;
    let headers: Vec<String> = rdr.headers().map_err(EngineError::decode)?.iter().map(str::to_owned).collect();
    let rows = rdr
        .records()
        .map(|r| r.map(|rec| rec.iter().map(infer_cell).collect()))
        .collect::<Result<Vec<Vec<Value>>, _>>()
        .map_err(EngineError::decode)?;
    Ok(Table { headers, rows })
}

/// First worksheet; first row is the header. (Sheet picker arrives with Phase 4 settings.)
fn spreadsheet(path: &Path) -> EngineResult<Table> {
    let mut wb = open_workbook_auto(path).map_err(EngineError::decode)?;
    let range = wb
        .worksheet_range_at(0)
        .ok_or_else(|| EngineError::Decode("workbook has no sheets".into()))?
        .map_err(EngineError::decode)?;

    let mut rows = range.rows();
    let headers = rows
        .next()
        .map(|r| r.iter().enumerate().map(|(i, c)| header_text(c, i)).collect())
        .unwrap_or_default();
    let rows = rows.map(|r| r.iter().map(sheet_cell).collect()).collect();
    Ok(Table { headers, rows })
}

fn header_text(c: &Data, i: usize) -> String {
    match c {
        Data::Empty => format!("column_{}", i + 1),
        other => other.to_string(),
    }
}

fn sheet_cell(c: &Data) -> Value {
    match c {
        Data::Int(i) => Value::Number((*i).into()),
        // Excel stores *every* number as f64; whole values come back as ints so
        // 709000 doesn't round-trip into "709000.0". 2^53 = last exactly-representable int.
        Data::Float(f) if f.fract() == 0.0 && f.abs() < 9_007_199_254_740_992.0 => Value::Number((*f as i64).into()),
        Data::Float(f) => Number::from_f64(*f).map_or(Value::Null, Value::Number),
        Data::String(s) => Value::String(s.clone()),
        Data::Bool(b) => Value::Bool(*b),
        // Excel stores dates as serial floats; without this they'd export as 45292.5.
        Data::DateTime(_) => c.as_datetime().map_or(Value::Null, |d| Value::String(d.to_string())),
        Data::DateTimeIso(s) | Data::DurationIso(s) => Value::String(s.clone()),
        Data::Error(_) | Data::Empty => Value::Null,
    }
}

/// First GFM pipe table in a Markdown document.
pub fn markdown_table(text: &str) -> EngineResult<Table> {
    let (mut headers, mut rows) = (Vec::new(), Vec::new());
    let (mut row, mut cell) = (Vec::<String>::new(), String::new());
    let mut in_table = false;

    for ev in Parser::new_ext(text, Options::ENABLE_TABLES) {
        match ev {
            Event::Start(Tag::Table(_)) => in_table = true,
            Event::Start(Tag::TableCell) => cell.clear(),
            Event::Text(t) | Event::Code(t) if in_table => cell.push_str(&t),
            Event::End(TagEnd::TableCell) => row.push(std::mem::take(&mut cell)),
            // pulldown-cmark emits head cells directly under TableHead (no TableRow).
            Event::End(TagEnd::TableHead) => headers = std::mem::take(&mut row),
            Event::End(TagEnd::TableRow) => rows.push(std::mem::take(&mut row).iter().map(|s| infer_cell(s)).collect()),
            Event::End(TagEnd::Table) => break,
            _ => {}
        }
    }
    if headers.is_empty() {
        return Err(EngineError::Unsupported("no Markdown table found in this document".into()));
    }
    Ok(Table { headers, rows })
}

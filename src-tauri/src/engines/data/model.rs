//! Intermediate representation: every reader yields a `Doc`, every writer takes one.
//! Two shapes cover everything — a grid (`Table`) or a tree (`serde_json::Value`).

use serde_json::{Map, Number, Value};

use crate::engines::error::{EngineError, EngineResult};
use crate::registry::rows_of;

#[derive(Debug, Clone, PartialEq)]
pub struct Table {
    pub headers: Vec<String>,
    /// Cells are scalar JSON values (Null/Bool/Number/String) so types survive the trip.
    pub rows: Vec<Vec<Value>>,
}

pub enum Doc {
    Table(Table),
    Tree(Value),
}

impl Doc {
    pub fn into_tree(self) -> Value {
        match self {
            Doc::Tree(v) => v,
            Doc::Table(t) => Value::Array(
                t.rows
                    .into_iter()
                    .map(|row| {
                        // preserve_order on serde_json keeps the column order here.
                        let obj: Map<String, Value> = t.headers.iter().cloned().zip(row).collect();
                        Value::Object(obj)
                    })
                    .collect(),
            ),
        }
    }

    pub fn into_table(self) -> EngineResult<Table> {
        match self {
            Doc::Table(t) => Ok(t),
            Doc::Tree(v) => {
                let records = rows_of(&v).ok_or_else(|| {
                    EngineError::Unsupported("this data is nested, not a flat list of records — it has no table form".into())
                })?;
                // Header = union of keys in first-seen order (records may be ragged).
                let mut headers: Vec<String> = Vec::new();
                for rec in records {
                    for k in rec.as_object().into_iter().flat_map(Map::keys) {
                        if !headers.contains(k) {
                            headers.push(k.clone());
                        }
                    }
                }
                let rows = records
                    .iter()
                    .map(|rec| headers.iter().map(|h| rec.get(h).cloned().unwrap_or(Value::Null)).collect())
                    .collect();
                Ok(Table { headers, rows })
            }
        }
    }
}

/// Text cell → typed JSON value, so CSV `42` becomes JSON `42`, not `"42"`.
/// Leading-zero strings (ZIP codes, IDs like `007`) deliberately stay strings.
pub fn infer_cell(s: &str) -> Value {
    let t = s.trim();
    if t.is_empty() {
        return Value::Null;
    }
    let leading_zero = t.len() > 1 && t.starts_with('0') && !t.starts_with("0.");
    if !leading_zero {
        if let Ok(i) = t.parse::<i64>() {
            return Value::Number(i.into());
        }
        if let Some(n) = t.parse::<f64>().ok().and_then(Number::from_f64) {
            return Value::Number(n);
        }
    }
    match t.to_ascii_lowercase().as_str() {
        "true" => Value::Bool(true),
        "false" => Value::Bool(false),
        _ => Value::String(s.to_owned()),
    }
}

pub fn cell_text(v: &Value) -> String {
    match v {
        Value::Null => String::new(),
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

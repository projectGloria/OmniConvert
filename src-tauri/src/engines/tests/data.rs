use std::fs;

use serde_json::{json, Value};

use super::{ctx, spec, to};
use crate::engines::data::convert;
use crate::engines::error::EngineError;
use crate::registry::Format::*;

const CSV: &str = "id,name,zip,active\n1,Ada,02134,true\n2,\"Lin, Grace\",90210,false\n";

#[test]
fn csv_json_csv_round_trip_keeps_types_order_and_leading_zeros() {
    let d = tempfile::tempdir().unwrap();
    let (csv, json, back) = (d.path().join("a.csv"), d.path().join("a.json"), d.path().join("b.csv"));
    fs::write(&csv, CSV).unwrap();

    convert(&spec(&csv, Csv, to(Json), json.clone()), &ctx()).unwrap();
    let v: Value = serde_json::from_str(&fs::read_to_string(&json).unwrap()).unwrap();
    assert_eq!(v[0], json!({ "id": 1, "name": "Ada", "zip": "02134", "active": true }));
    // preserve_order: keys come out in column order, not alphabetical.
    let keys: Vec<&String> = v[1].as_object().unwrap().keys().collect();
    assert_eq!(keys, ["id", "name", "zip", "active"]);

    convert(&spec(&json, Json, to(Csv), back.clone()), &ctx()).unwrap();
    assert_eq!(fs::read_to_string(back).unwrap(), CSV);
}

#[test]
fn nested_json_refuses_table_output() {
    let d = tempfile::tempdir().unwrap();
    let src = d.path().join("n.json");
    fs::write(&src, r#"{"cfg":{"deep":{"x":1}}}"#).unwrap();
    let err = convert(&spec(&src, Json, to(Csv), d.path().join("n.csv")), &ctx()).unwrap_err();
    assert!(matches!(err, EngineError::Unsupported(_)), "{err}");
}

#[test]
fn markdown_table_to_xlsx_and_back() {
    let d = tempfile::tempdir().unwrap();
    let (md, xlsx, csv) = (d.path().join("t.md"), d.path().join("t.xlsx"), d.path().join("t.csv"));
    fs::write(&md, "# Report\n\n| city | pop |\n|---|--:|\n| Oslo | 709000 |\n| Bergen | 291000 |\n").unwrap();

    convert(&spec(&md, Md, to(Xlsx), xlsx.clone()), &ctx()).unwrap();
    convert(&spec(&xlsx, Xlsx, to(Csv), csv.clone()), &ctx()).unwrap();
    assert_eq!(fs::read_to_string(csv).unwrap(), "city,pop\nOslo,709000\nBergen,291000\n");
}

#[test]
fn records_survive_xml_and_toml() {
    let d = tempfile::tempdir().unwrap();
    let src = d.path().join("r.json");
    fs::write(&src, r#"[{"sku":"A-1","qty":3,"note":null},{"sku":"B-2","qty":5,"note":"fragile"}]"#).unwrap();

    let (xml, json2) = (d.path().join("r.xml"), d.path().join("r2.json"));
    convert(&spec(&src, Json, to(Xml), xml.clone()), &ctx()).unwrap();
    convert(&spec(&xml, Xml, to(Json), json2.clone()), &ctx()).unwrap();
    let v: Value = serde_json::from_str(&fs::read_to_string(&json2).unwrap()).unwrap();
    // XML read unwraps <root>; the repeated elements become the record array.
    assert_eq!(v["item"][1], json!({ "sku": "B-2", "qty": 5, "note": "fragile" }));

    // TOML: top-level array becomes [[rows]], nulls are dropped (TOML has no null).
    let toml_out = d.path().join("r.toml");
    convert(&spec(&src, Json, to(Toml), toml_out.clone()), &ctx()).unwrap();
    let t = fs::read_to_string(toml_out).unwrap();
    assert!(t.contains("[[rows]]") && !t.contains("note = \"\""), "{t}");
}

#[test]
fn xml_writer_sanitizes_names_and_escapes() {
    let d = tempfile::tempdir().unwrap();
    let src = d.path().join("w.json");
    fs::write(&src, r#"{"1st key":"a<b & c","@id":7}"#).unwrap();
    let out = d.path().join("w.xml");
    convert(&spec(&src, Json, to(Xml), out.clone()), &ctx()).unwrap();
    let x = fs::read_to_string(out).unwrap();
    assert!(x.contains(r#"<root id="7">"#) && x.contains("<_1st_key>a&lt;b &amp; c</_1st_key>"), "{x}");
}

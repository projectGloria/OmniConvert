//! Pure-Rust routes (data, image, archive). Always run: no sidecars involved.

use std::fs;

use omniconvert_lib::testkit::{Format::*, Harness, JobSettings, TargetKind};
use serde_json::Value;

use crate::check::{assert_output, csv_rows};
use crate::fixtures::{Fixtures, ARCHIVE_ENTRIES, PNG_SIDE};
use crate::{run, run_with, to};

#[test]
fn csv_to_json_to_csv_keeps_every_row() {
    let (fx, h) = (Fixtures::new(), Harness::new());
    let csv = fx.csv();
    let json = run(&h, &csv, to(Json));
    assert_output(&json, Json);
    // The JSON leg is a different file name, so the round-trip doesn't overwrite the source.
    let renamed = fx.root.join("roundtrip.json");
    fs::rename(&json, &renamed).unwrap();
    let back = run(&h, &renamed, to(Csv));
    assert_output(&back, Csv);
    // Row equality, not byte equality: quoting style is the writer's call, the data isn't.
    assert_eq!(csv_rows(&back), csv_rows(&csv));
}

#[test]
fn json_to_yaml_to_json_is_value_identical() {
    let (fx, h) = (Fixtures::new(), Harness::new());
    let json = fx.json();
    let yaml = run(&h, &json, to(Yaml));
    assert_output(&yaml, Yaml);
    let renamed = fx.root.join("again.yaml");
    fs::rename(&yaml, &renamed).unwrap();
    let back = run(&h, &renamed, to(Json));
    let parse = |p| serde_json::from_str::<Value>(&fs::read_to_string(p).unwrap()).unwrap();
    assert_eq!(parse(&back), parse(&json));
}

#[test]
fn json_fans_out_to_every_tabular_and_tree_format() {
    let (fx, h) = (Fixtures::new(), Harness::new());
    let json = fx.json();
    for f in [Csv, Tsv, Xlsx, MdTable, Yaml, Xml] {
        assert_output(&run(&h, &json, to(f)), f);
    }
}

#[test]
fn xlsx_reads_back_the_same_table() {
    let (fx, h) = (Fixtures::new(), Harness::new());
    let xlsx = run(&h, &fx.csv(), to(Xlsx));
    let back = run(&h, &xlsx, to(Csv)); // people.csv now exists → planner picks a free name
    assert_ne!(back, fx.root.join("people.csv"), "planner overwrote the source");
    assert_eq!(csv_rows(&back), csv_rows(&fx.root.join("people.csv")));
}

#[test]
fn markdown_table_is_readable_as_data() {
    let (fx, h) = (Fixtures::new(), Harness::new());
    let csv = run(&h, &fx.md(), to(Csv));
    assert_eq!(csv_rows(&csv), [["Region", "Units"], ["North", "120"], ["South", "95"]]);
}

#[test]
fn png_reaches_every_image_target_and_pdf() {
    let (fx, h) = (Fixtures::new(), Harness::new());
    let (png, _) = fx.png();
    for f in [Jpg, Webp, Avif, Ico, Bmp, Tiff, Gif, Pdf] {
        assert_output(&run(&h, &png, to(f)), f);
    }
}

#[test]
fn png_to_lossless_webp_and_back_is_pixel_identical() {
    let (fx, h) = (Fixtures::new(), Harness::new());
    let (png, original) = fx.png();
    let settings = JobSettings { quality: Some(100), ..JobSettings::default() }; // 100 ⇒ lossless
    let webp = run_with(&h, &png, to(Webp), settings);
    let back = run(&h, &webp, to(Png));
    assert_output(&back, Png);

    let decoded = image::open(&back).unwrap().to_rgba8();
    assert_eq!(decoded.dimensions(), (PNG_SIDE, PNG_SIDE));
    for (got, want) in decoded.pixels().zip(original.pixels()) {
        assert_eq!(got.0[3], want.0[3], "alpha changed");
        // libwebp zeroes RGB under alpha=0 (invisible); everything visible must survive.
        if want.0[3] > 0 {
            assert_eq!(got.0, want.0);
        }
    }
}

#[test]
fn archives_repack_and_extract_byte_identical() {
    let (fx, h) = (Fixtures::new(), Harness::new());
    let zip = fx.zip();
    for f in [SevenZ, Tar, Tgz, Txz] {
        let packed = run(&h, &zip, to(f));
        assert_output(&packed, f);
        let dir = run(&h, &packed, TargetKind::Extract);
        for (name, body) in ARCHIVE_ENTRIES {
            let got = fs::read(dir.join(name)).unwrap_or_else(|e| panic!("{f:?}: {name}: {e}"));
            assert_eq!(&got, body, "{f:?}: {name}");
        }
    }
}

#[test]
fn matrix_refusal_is_reported_not_attempted() {
    let (fx, h) = (Fixtures::new(), Harness::new());
    // Audio from a spreadsheet: the picker never offers it, and the harness must agree.
    let err = h.convert(&fx.csv(), to(Mp3), JobSettings::default()).unwrap_err();
    assert!(err.to_string().contains("doesn't offer"), "{err}");
}

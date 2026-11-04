use std::collections::HashSet;
use std::path::Path;

use serde_json::json;

use super::formats::{Category, Format, Format::*};
use super::matrix::targets_for;
use super::route::{route_for, EngineRoute};
use super::shape::{rows_of, Tabular, Traits};
use super::targets::TargetKind;

const ALL: &[Format] = &[
    Mp4, Mkv, Mov, Avi, Webm, Flv, Mp3, Wav, Flac, Aac, Ogg, M4a, Opus, Png, Jpg, Webp, Avif, Ico,
    Bmp, Tiff, Gif, Svg, Pdf, Docx, Md, Html, Txt, Rtf, Epub, Xlsx, Xls, Ods, Csv, Tsv, Json, Yaml,
    Toml, Xml, MdTable, Zip, SevenZ, Tar, Gz, Tgz, Xz, Txz,
];

/// Every combination of content traits the matrix branches on.
fn trait_variants() -> Vec<Traits> {
    let mut v = Vec::new();
    for animated in [false, true] {
        for md_table in [false, true] {
            for tabular in [Tabular::Yes, Tabular::No, Tabular::Unknown] {
                v.push(Traits { animated, md_table, tabular });
            }
        }
    }
    v
}

#[test]
fn id_matches_serde_wire_name() {
    for &f in ALL {
        assert_eq!(json!(f), json!(f.id()), "{f:?}: id() drifted from serde name");
    }
}

#[test]
fn every_readable_format_has_targets() {
    // MdTable is output-only; it is never detected as a source.
    for &f in ALL.iter().filter(|&&f| f != MdTable) {
        for t in trait_variants() {
            assert!(!targets_for(f, &t).is_empty(), "{f:?} has no targets with {t:?}");
        }
    }
}

#[test]
fn never_targets_itself_and_ids_are_unique() {
    for &f in ALL {
        for t in trait_variants() {
            let targets = targets_for(f, &t);
            let mut seen = HashSet::new();
            for target in &targets {
                assert_ne!(target.kind, TargetKind::Convert(f), "{f:?} offers itself");
                assert!(seen.insert(target.id.clone()), "{f:?}: duplicate target id {}", target.id);
            }
        }
    }
}

#[test]
fn svg_and_readonly_formats_are_never_outputs() {
    for &f in ALL {
        for t in trait_variants() {
            for target in targets_for(f, &t) {
                for banned in [Svg, Xls, Ods] {
                    assert_ne!(target.kind, TargetKind::Convert(banned), "{f:?} offers {banned:?}");
                }
            }
        }
    }
}

#[test]
fn pdf_never_routes_to_pandoc() {
    for t in targets_for(Pdf, &trait_variants()[0]) {
        assert_eq!(route_for(Pdf, t.kind), EngineRoute::Pdf, "PDF → {} hit pandoc", t.id);
    }
}

#[test]
fn routes_are_sane() {
    assert_eq!(route_for(Mp4, TargetKind::Convert(Mp3)), EngineRoute::Media);
    assert_eq!(route_for(Avif, TargetKind::Convert(Png)), EngineRoute::MediaThenImage);
    assert_eq!(route_for(Svg, TargetKind::Convert(Pdf)), EngineRoute::Svg);
    assert_eq!(route_for(Gif, TargetKind::Convert(Mp4)), EngineRoute::Media);
    assert_eq!(route_for(Md, TargetKind::Convert(Csv)), EngineRoute::Data);
    assert_eq!(route_for(Md, TargetKind::Convert(Docx)), EngineRoute::Document);
    assert_eq!(route_for(Zip, TargetKind::Extract), EngineRoute::Archive);
}

#[test]
fn tabular_traits_gate_grid_targets() {
    let grid = |tabular| {
        targets_for(Json, &Traits { animated: false, md_table: false, tabular })
            .iter()
            .any(|t| t.kind == TargetKind::Convert(Csv))
    };
    assert!(grid(Tabular::Yes));
    assert!(!grid(Tabular::No));
    assert!(grid(Tabular::Unknown));
}

#[test]
fn animated_gif_gets_video_targets() {
    let anim = Traits { animated: true, md_table: false, tabular: Tabular::Unknown };
    let groups: HashSet<Category> = targets_for(Gif, &anim).iter().map(|t| t.group).collect();
    assert!(groups.contains(&Category::Video));
}

#[test]
fn rows_of_accepts_flat_records_only() {
    assert!(rows_of(&json!([{ "a": 1 }, { "a": 2, "b": "x" }])).is_some());
    assert!(rows_of(&json!({ "rows": [{ "a": 1 }] })).is_some()); // TOML [[rows]] shape
    assert!(rows_of(&json!([{ "a": { "nested": 1 } }])).is_none());
    assert!(rows_of(&json!([1, 2, 3])).is_none());
    assert!(rows_of(&json!([])).is_none());
    assert!(rows_of(&json!({ "a": [], "b": [] })).is_none());
}

#[test]
fn extension_parsing_handles_aliases_and_double_ext() {
    assert_eq!(Format::from_path(Path::new(r"C:\x\backup.TAR.GZ")), Some(Tgz));
    assert_eq!(Format::from_path(Path::new("photo.JPEG")), Some(Jpg));
    assert_eq!(Format::from_path(Path::new("cfg.yml")), Some(Yaml));
    assert_eq!(Format::from_path(Path::new("no_extension")), None);
}

#[test]
fn magic_bytes_override_mislabelled_image() {
    let path = std::env::temp_dir().join(format!("omni-detect-{}.png", std::process::id()));
    // Minimal JPEG SOI + APP0/JFIF header — enough for `infer` to say "jpg".
    std::fs::write(&path, [0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, b'J', b'F', b'I', b'F', 0x00]).unwrap();
    let d = super::detect::detect(&path);
    std::fs::remove_file(&path).ok();
    assert_eq!(d.format, Some(Jpg));
    assert_eq!(d.detected_as, Some(Jpg));
}

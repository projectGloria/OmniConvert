//! Merge to PDF + settings plumbing that isn't a single-file conversion.

use image::{Rgba, RgbaImage};

use super::ctx;
use crate::engines::error::EngineError;
use crate::engines::image::merge::merge_pdf;
use crate::settings::JobSettings;
use crate::registry::Format;

#[test]
fn merges_mixed_images_into_one_pdf_in_order() {
    let d = tempfile::tempdir().unwrap();
    let (png, jpg, svg) = (d.path().join("a.png"), d.path().join("b.jpg"), d.path().join("c.svg"));
    RgbaImage::from_pixel(40, 30, Rgba([200, 10, 10, 128])).save(&png).unwrap();
    image::RgbImage::from_pixel(20, 50, image::Rgb([0, 90, 200])).save(&jpg).unwrap();
    std::fs::write(&svg, r#"<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"><rect width="10" height="10"/></svg>"#).unwrap();

    let out = d.path().join("m.pdf");
    let inputs = vec![(png, Format::Png), (jpg, Format::Jpg), (svg, Format::Svg)];
    merge_pdf(&inputs, &JobSettings::default(), &out, &ctx()).unwrap();

    let doc = lopdf::Document::load(&out).unwrap();
    let pages = doc.get_pages();
    assert_eq!(pages.len(), 3);
    // Page 2 is the 20×50 JPEG at 0.75 pt/px ⇒ 15×37.5 pt: proves order is preserved.
    let page2 = doc.get_dictionary(pages[&2]).unwrap();
    let mb = page2.get(b"MediaBox").unwrap().as_array().unwrap();
    assert_eq!(mb[2].as_float().unwrap(), 15.0);
    assert_eq!(mb[3].as_float().unwrap(), 37.5);
}

#[test]
fn merge_honours_cancellation() {
    let d = tempfile::tempdir().unwrap();
    let png = d.path().join("a.png");
    RgbaImage::new(4, 4).save(&png).unwrap();
    let c = ctx();
    c.cancel.cancel();
    let r = merge_pdf(&[(png, Format::Png)], &JobSettings::default(), &d.path().join("m.pdf"), &c);
    assert!(matches!(r, Err(EngineError::Cancelled)));
}

#[test]
fn background_parses_hex_and_falls_back_to_white() {
    let mut s = JobSettings { background: Some("#1e1E24".into()), ..Default::default() };
    assert_eq!(s.background_rgb(), [0x1e, 0x1e, 0x24]);
    for bad in ["1e1e24", "#12345", "#zzzzzz", "#é1234"] {
        s.background = Some(bad.into());
        assert_eq!(s.background_rgb(), [255, 255, 255], "{bad}");
    }
}

#[test]
fn tool_errors_split_tail_into_detail() {
    let e = EngineError::Tool { tool: "ffmpeg".into(), code: 1, tail: "Invalid data".into() };
    assert_eq!(e.into_parts(), ("ffmpeg failed (exit 1)".into(), Some("Invalid data".into())));
    assert_eq!(EngineError::Cancelled.into_parts(), ("cancelled".into(), None));
}

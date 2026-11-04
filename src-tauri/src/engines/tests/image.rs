use std::fs;
use std::path::Path;

use image::{DynamicImage, Rgba, RgbaImage};

use super::{ctx, spec, to};
use crate::engines::image::{convert, decode};
use crate::registry::Format::*;

/// 64×48 gradient with a fully transparent left column.
fn fixture(path: &Path) -> RgbaImage {
    let img = RgbaImage::from_fn(64, 48, |x, y| Rgba([(x * 4) as u8, (y * 5) as u8, 128, if x == 0 { 0 } else { 255 }]));
    img.save(path).unwrap();
    img
}

#[test]
fn png_to_lossless_webp_is_pixel_identical() {
    let d = tempfile::tempdir().unwrap();
    let (png, webp) = (d.path().join("a.png"), d.path().join("a.webp"));
    let original = fixture(&png);

    let mut s = spec(&png, Png, to(Webp), webp.clone());
    s.settings.quality = Some(100); // 100 ⇒ lossless path
    convert(&s, &ctx()).unwrap();
    // libwebp lossless zeroes the RGB of fully transparent pixels (invisible anyway),
    // so "identical" means: same alpha everywhere, same RGB wherever it's visible.
    let decoded = decode(&webp).unwrap().to_rgba8();
    for (a, b) in decoded.pixels().zip(original.pixels()) {
        assert_eq!(a.0[3], b.0[3]);
        if b.0[3] > 0 {
            assert_eq!(a.0, b.0);
        }
    }
}

#[test]
fn jpg_flattens_alpha_onto_white_not_black() {
    let d = tempfile::tempdir().unwrap();
    let (png, jpg) = (d.path().join("a.png"), d.path().join("a.jpg"));
    fixture(&png);
    convert(&spec(&png, Png, to(Jpg), jpg.clone()), &ctx()).unwrap();
    let px = decode(&jpg).unwrap().to_rgb8().get_pixel(0, 10).0;
    assert!(px.iter().all(|&c| c > 230), "transparent pixel became {px:?}");
}

#[test]
fn resize_respects_max_dimension_and_aspect() {
    let d = tempfile::tempdir().unwrap();
    let (png, out) = (d.path().join("a.png"), d.path().join("small.png"));
    fixture(&png);
    let mut s = spec(&png, Png, to(Png), out.clone());
    s.settings.max_dimension = Some(32);
    convert(&s, &ctx()).unwrap();
    let img = decode(&out).unwrap();
    assert_eq!((img.width(), img.height()), (32, 24));
}

#[test]
fn ico_bmp_tiff_gif_avif_all_encode_and_decode() {
    let d = tempfile::tempdir().unwrap();
    let png = d.path().join("a.png");
    fixture(&png);
    for f in [Ico, Bmp, Tiff, Gif, Avif] {
        let out = d.path().join(format!("a.{}", f.ext()));
        convert(&spec(&png, Png, to(f), out.clone()), &ctx()).unwrap_or_else(|e| panic!("{f:?}: {e}"));
        if f != Avif {
            // AVIF decode needs dav1d (ffmpeg path) — only check it's a real AVIF container.
            assert!(decode(&out).is_ok(), "{f:?} output doesn't decode");
        } else {
            assert_eq!(&fs::read(&out).unwrap()[4..12], b"ftypavif");
        }
    }
}

#[test]
fn jpeg_to_pdf_embeds_original_bytes() {
    let d = tempfile::tempdir().unwrap();
    let (jpg, pdf) = (d.path().join("p.jpg"), d.path().join("p.pdf"));
    DynamicImage::ImageRgba8(fixture(&d.path().join("x.png"))).to_rgb8().save(&jpg).unwrap();

    convert(&spec(&jpg, Jpg, to(Pdf), pdf.clone()), &ctx()).unwrap();
    let bytes = fs::read(&pdf).unwrap();
    assert!(bytes.starts_with(b"%PDF-1.7"));
    let jpeg = fs::read(&jpg).unwrap();
    // DCTDecode passthrough ⇒ the exact JPEG byte stream appears inside the PDF.
    assert!(bytes.windows(jpeg.len()).any(|w| w == jpeg.as_slice()), "JPEG was re-encoded");
}

#[test]
fn svg_rasterizes_with_text_and_ico_upscale() {
    let d = tempfile::tempdir().unwrap();
    let svg = d.path().join("i.svg");
    fs::write(&svg, r##"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24"><circle cx="12" cy="12" r="10" fill="#6366F1"/></svg>"##).unwrap();

    let png = d.path().join("i.png");
    convert(&spec(&svg, Svg, to(Png), png.clone()), &ctx()).unwrap();
    let img = decode(&png).unwrap().to_rgba8();
    assert_eq!((img.width(), img.height()), (24, 24));
    assert_eq!(img.get_pixel(12, 12).0, [0x63, 0x66, 0xF1, 255]); // demultiplied correctly

    let ico = d.path().join("i.ico");
    convert(&spec(&svg, Svg, to(Ico), ico.clone()), &ctx()).unwrap();
    assert_eq!(decode(&ico).unwrap().width(), 256); // largest frame = 256 master
}

//! Images → PDF with lopdf (PLAN D6). One page per image, page size = image size at 96 DPI.
//! Takes a list so Phase 4's "Merge to PDF" batch action reuses it unchanged.

use std::io::Write;
use std::path::Path;

use flate2::write::ZlibEncoder;
use flate2::Compression;
use image::DynamicImage;
use lopdf::content::{Content, Operation};
use lopdf::{dictionary, Document, Object, ObjectId, Stream};

use crate::engines::error::{EngineError, EngineResult};

pub enum PdfImage {
    /// Embedded verbatim (DCTDecode) — the PDF *is* the original JPEG, zero re-encode.
    Jpeg { bytes: Vec<u8>, width: u32, height: u32, gray: bool },
    /// Decoded pixels, Flate-compressed RGB with an optional alpha SMask.
    Raster(DynamicImage),
}

/// px → pt at 96 DPI (72 pt per inch). A 1920px-wide screenshot becomes a 20" page —
/// same physical size a browser would print it at.
const PX_TO_PT: f32 = 0.75;

pub fn write(images: Vec<PdfImage>, out: &Path) -> EngineResult<()> {
    let mut doc = Document::with_version("1.7");
    let pages_id = doc.new_object_id();
    let mut kids: Vec<Object> = Vec::with_capacity(images.len());

    for img in images {
        let (w, h, xobject) = image_xobject(&mut doc, img)?;
        let (pw, ph) = (w as f32 * PX_TO_PT, h as f32 * PX_TO_PT);

        // Content stream: scale the unit-square image to the full page and paint it.
        let content = Content {
            operations: vec![
                Operation::new("q", vec![]),
                Operation::new("cm", vec![pw.into(), 0.into(), 0.into(), ph.into(), 0.into(), 0.into()]),
                Operation::new("Do", vec![Object::Name(b"Im0".to_vec())]),
                Operation::new("Q", vec![]),
            ],
        };
        let content_id = doc.add_object(Stream::new(dictionary! {}, content.encode().map_err(EngineError::encode)?));
        let page_id = doc.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "MediaBox" => vec![0.into(), 0.into(), pw.into(), ph.into()],
            "Contents" => content_id,
            "Resources" => dictionary! { "XObject" => dictionary! { "Im0" => xobject } },
        });
        kids.push(page_id.into());
    }

    let count = kids.len() as i64;
    doc.objects.insert(pages_id, Object::Dictionary(dictionary! { "Type" => "Pages", "Kids" => kids, "Count" => count }));
    let catalog_id = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages_id });
    doc.trailer.set("Root", catalog_id);

    doc.save(out).map_err(EngineError::encode)?;
    Ok(())
}

fn image_xobject(doc: &mut Document, img: PdfImage) -> EngineResult<(u32, u32, ObjectId)> {
    match img {
        PdfImage::Jpeg { bytes, width, height, gray } => {
            let dict = dictionary! {
                "Type" => "XObject", "Subtype" => "Image",
                "Width" => i64::from(width), "Height" => i64::from(height),
                "ColorSpace" => if gray { "DeviceGray" } else { "DeviceRGB" },
                "BitsPerComponent" => 8, "Filter" => "DCTDecode",
            };
            Ok((width, height, doc.add_object(Stream::new(dict, bytes))))
        }
        PdfImage::Raster(img) => {
            let (w, h) = (img.width(), img.height());
            let mut dict = dictionary! {
                "Type" => "XObject", "Subtype" => "Image",
                "Width" => i64::from(w), "Height" => i64::from(h),
                "ColorSpace" => "DeviceRGB", "BitsPerComponent" => 8, "Filter" => "FlateDecode",
            };
            if img.color().has_alpha() {
                // PDF has no RGBA images: alpha travels as a separate grayscale soft mask.
                let alpha: Vec<u8> = img.to_rgba8().pixels().map(|p| p.0[3]).collect();
                let smask = dictionary! {
                    "Type" => "XObject", "Subtype" => "Image",
                    "Width" => i64::from(w), "Height" => i64::from(h),
                    "ColorSpace" => "DeviceGray", "BitsPerComponent" => 8, "Filter" => "FlateDecode",
                };
                let smask_id = doc.add_object(Stream::new(smask, zlib(&alpha)?));
                dict.set("SMask", smask_id);
            }
            let rgb = img.to_rgb8();
            Ok((w, h, doc.add_object(Stream::new(dict, zlib(rgb.as_raw())?))))
        }
    }
}

fn zlib(data: &[u8]) -> EngineResult<Vec<u8>> {
    let mut enc = ZlibEncoder::new(Vec::with_capacity(data.len() / 2), Compression::default());
    enc.write_all(data)?;
    Ok(enc.finish()?)
}

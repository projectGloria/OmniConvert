//! Routes that need ffmpeg / pandoc / typst / pdfium. `#[ignore]`d by default; run them
//! with `npm run test:e2e` once `npm run sidecars` has populated src-tauri/binaries.

use std::fs;
use std::io::Read;

use omniconvert_lib::testkit::{Format::*, Harness};

use crate::check::assert_output;
use crate::fixtures::{Fixtures, WAV_RATE};
use crate::{run, to};

const NEEDS: &str = "needs sidecars: npm run sidecars";

#[test]
#[ignore = "needs sidecars: npm run sidecars"]
fn wav_to_flac_and_back_is_sample_exact() {
    let (fx, h) = (Fixtures::new(), Harness::new());
    let (wav, samples) = fx.wav();
    let flac = run(&h, &wav, to(Flac));
    assert_output(&flac, Flac);
    let back = run(&h, &flac, to(Wav)); // tone.wav exists → lands as a new name
    assert_output(&back, Wav);

    let mut r = hound::WavReader::open(&back).expect(NEEDS);
    assert_eq!(r.spec().sample_rate, WAV_RATE);
    let got: Vec<i16> = r.samples::<i16>().map(Result::unwrap).collect();
    assert_eq!(got, samples, "FLAC is lossless; any drift is a pipeline bug");
}

#[test]
#[ignore = "needs sidecars: npm run sidecars"]
fn wav_to_lossy_audio_formats() {
    let (fx, h) = (Fixtures::new(), Harness::new());
    let (wav, _) = fx.wav();
    for f in [Mp3, Opus, Ogg] {
        let out = run(&h, &wav, to(f));
        assert_output(&out, f);
        // 1 s of audio at ≥96 kbps can't be under ~4 KB unless the encode silently failed.
        assert!(fs::metadata(&out).unwrap().len() > 4_000, "{f:?} suspiciously small");
    }
}

#[test]
#[ignore = "needs sidecars: npm run sidecars"]
fn markdown_to_docx_keeps_headings_and_table() {
    let (fx, h) = (Fixtures::new(), Harness::new());
    let docx = run(&h, &fx.md(), to(Docx));
    assert_output(&docx, Docx);
    let mut zip = zip::ZipArchive::new(fs::File::open(&docx).unwrap()).unwrap();
    let mut xml = String::new();
    zip.by_name("word/document.xml").unwrap().read_to_string(&mut xml).unwrap();
    assert!(xml.contains("Quarterly Report"), "heading text missing");
    assert!(xml.contains("<w:tbl>"), "pipe table didn't become a Word table");
}

#[test]
#[ignore = "needs sidecars: npm run sidecars"]
fn markdown_to_html_is_standalone() {
    let (fx, h) = (Fixtures::new(), Harness::new());
    let html = run(&h, &fx.md(), to(Html));
    assert_output(&html, Html);
    let s = fs::read_to_string(&html).unwrap();
    for needle in ["<html", "<h1", "<table", "Quarterly Report"] {
        assert!(s.contains(needle), "missing {needle}");
    }
}

/// MD → PDF (pandoc + typst), then that PDF → text and → page PNGs (pdfium): proves the
/// typeset PDF has a real text layer, and covers the directory-output (`_pages`) path.
#[test]
#[ignore = "needs sidecars: npm run sidecars"]
fn markdown_to_pdf_then_pdfium_reads_it_back() {
    let (fx, h) = (Fixtures::new(), Harness::new());
    let pdf = run(&h, &fx.md(), to(Pdf));
    assert_output(&pdf, Pdf);

    let txt = run(&h, &pdf, to(Txt));
    let text = fs::read_to_string(&txt).unwrap();
    assert!(text.contains("Quarterly Report") && text.contains("North"), "text layer: {text:?}");

    // Second pdfium job in the same process: regression test for the bind-once bug.
    let pages = run(&h, &pdf, to(Png));
    assert!(pages.is_dir(), "PDF → PNG should produce a folder");
    assert_output(&pages.join("page-001.png"), Png);
}

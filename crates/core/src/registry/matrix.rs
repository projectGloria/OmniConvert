//! The ConversionMatrix: (source format, content traits) → legal targets.
//! Single source of truth — the frontend never decides what's convertible.

use super::formats::{
    Category, Format, ARCHIVE_OUT, AUDIO_OUT, DOC_OUT, IMAGE_OUT, TABULAR_OUT, TREE_OUT, VIDEO_OUT,
};
use super::shape::{Tabular, Traits};
use super::targets::{Target, TargetKind};
use Format::*;

pub fn targets_for(src: Format, t: &Traits) -> Vec<Target> {
    let mut out = Vec::new();
    match src.category() {
        Category::Video => video(src, &mut out),
        Category::Audio => push_formats(&mut out, AUDIO_OUT, src),
        Category::Image => image(src, t, &mut out),
        Category::Document => document(src, t, &mut out),
        Category::Data => data(src, t, &mut out),
        Category::Archive => archive(src, &mut out),
    }
    out
}

/// Pushes a plain Convert target for every format in `set` except the source itself.
fn push_formats(out: &mut Vec<Target>, set: &[Format], src: Format) {
    out.extend(set.iter().filter(|&&f| f != src).map(|&f| Target::convert(f)));
}

fn video(src: Format, out: &mut Vec<Target>) {
    push_formats(out, VIDEO_OUT, src);
    push_formats(out, AUDIO_OUT, src); // audio extraction: group = Audio via Target::convert
    out.push(Target::new(TargetKind::AnimatedGif, "Animated GIF", Category::Image));
    out.push(Target::new(TargetKind::AnimatedWebp, "Animated WebP", Category::Image));
    out.push(
        Target::new(TargetKind::FramesZip, "Frames → ZIP (PNG)", Category::Image)
            .with_note("One PNG per frame at the chosen FPS"),
    );
}

fn image(src: Format, t: &Traits, out: &mut Vec<Target>) {
    if src == Gif && t.animated {
        // Animated GIF behaves like a tiny video: re-encode via ffmpeg.
        for f in [Mp4, Webm] {
            out.push(Target::convert(f));
        }
        out.push(Target::new(TargetKind::AnimatedWebp, "Animated WebP", Category::Image));
        for f in [Png, Jpg] {
            out.push(Target::convert(f).with_note("First frame only"));
        }
        return;
    }
    // SVG is never an output (raster → vector is tracing, not conversion).
    push_formats(out, IMAGE_OUT, src);
    out.push(Target::convert(Pdf));
}

fn document(src: Format, t: &Traits, out: &mut Vec<Target>) {
    if src == Pdf {
        // Pandoc can't read PDF. Only what pdfium can extract is honest to offer.
        out.push(Target::convert(Png).with_note("One image per page"));
        out.push(Target::convert(Jpg).with_note("One image per page"));
        out.push(Target::convert(Txt).with_note("Text layer only, no layout"));
        return;
    }
    for &f in DOC_OUT.iter().filter(|&&f| f != src) {
        let target = Target::convert(f);
        out.push(if f == Pdf { target.with_note("Re-typeset with Typst; layout may differ") } else { target });
    }
    if src == Md && t.md_table {
        // A Markdown doc containing a pipe table can also be read as data (first table).
        for &f in TABULAR_OUT.iter().chain(TREE_OUT).filter(|&&f| f != MdTable) {
            out.push(Target::convert(f).with_note("First table in the document"));
        }
    }
}

fn data(src: Format, t: &Traits, out: &mut Vec<Target>) {
    let is_tree = TREE_OUT.contains(&src);
    if is_tree {
        push_formats(out, TREE_OUT, src);
        match t.tabular {
            Tabular::Yes => push_formats(out, TABULAR_OUT, src),
            Tabular::No => {} // nested data has no faithful grid form — don't offer it
            Tabular::Unknown => out.extend(
                TABULAR_OUT.iter().map(|&f| Target::convert(f).with_note("Needs an array of flat records")),
            ),
        }
    } else {
        // Grid sources (CSV/TSV/XLSX/XLS/ODS) convert to every grid and tree format.
        // XLS/ODS are read-only, so they never appear as outputs.
        push_formats(out, TABULAR_OUT, src);
        push_formats(out, TREE_OUT, src);
    }
}

fn archive(src: Format, out: &mut Vec<Target>) {
    out.push(Target::new(TargetKind::Extract, "Extract to folder", Category::Archive));
    match src {
        // Single-stream compressors: decompressing *is* the extraction; repack into real archives.
        Gz | Xz => push_formats(out, &[Zip, SevenZ], src),
        _ => push_formats(out, ARCHIVE_OUT, src),
    }
}

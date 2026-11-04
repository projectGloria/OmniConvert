//! Output assertions: right container (magic bytes), and our own detector agrees —
//! i.e. a finished output can be dropped straight back into the queue.

use std::fs;
use std::path::Path;

use omniconvert_lib::testkit::{inspect, Format};

/// Asserts `path` is a non-empty file whose leading bytes match `fmt`'s container.
pub fn assert_output(path: &Path, fmt: Format) {
    let bytes = fs::read(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    assert!(!bytes.is_empty(), "{} is empty", path.display());
    assert!(magic_ok(&bytes, fmt), "{} doesn't look like {fmt:?}: {:02X?}", path.display(), &bytes[..bytes.len().min(16)]);

    // A Markdown table output is a `.md` file, so it re-detects as a Markdown document.
    let expect = if fmt == Format::MdTable { Format::Md } else { fmt };
    assert_eq!(inspect(path).format, Some(expect), "{} re-detected wrong", path.display());
}

fn magic_ok(b: &[u8], fmt: Format) -> bool {
    use Format::*;
    let at = |off: usize, sig: &[u8]| b.get(off..off + sig.len()) == Some(sig);
    match fmt {
        Png => at(0, b"\x89PNG\r\n\x1a\n"),
        Jpg => at(0, &[0xFF, 0xD8, 0xFF]),
        Webp => at(0, b"RIFF") && at(8, b"WEBP"),
        Avif => at(4, b"ftypavif") || at(4, b"ftypavis"),
        Ico => at(0, &[0, 0, 1, 0]),
        Bmp => at(0, b"BM"),
        Tiff => at(0, b"II*\0") || at(0, b"MM\0*"),
        Gif => at(0, b"GIF89a") || at(0, b"GIF87a"),
        Pdf => at(0, b"%PDF-"),
        // OOXML / ODF / EPUB are all ZIP containers.
        Zip | Xlsx | Docx | Ods | Epub => at(0, b"PK\x03\x04"),
        SevenZ => at(0, &[0x37, 0x7A, 0xBC, 0xAF, 0x27, 0x1C]),
        Gz | Tgz => at(0, &[0x1F, 0x8B]),
        Xz | Txz => at(0, &[0xFD, b'7', b'z', b'X', b'Z', 0]),
        Tar => at(257, b"ustar"),
        Wav => at(0, b"RIFF") && at(8, b"WAVE"),
        Flac => at(0, b"fLaC"),
        Ogg => at(0, b"OggS"),
        // Opus is Ogg-framed; the first page carries the OpusHead ID header.
        Opus => at(0, b"OggS") && b.windows(8).take(64).any(|w| w == b"OpusHead"),
        // ffmpeg writes an ID3v2 tag first; a bare stream starts with an MPEG frame sync.
        Mp3 => at(0, b"ID3") || (b[0] == 0xFF && b.get(1).is_some_and(|x| x & 0xE0 == 0xE0)),
        // Text formats have no magic; "is UTF-8" is the meaningful container check.
        Csv | Tsv | Json | Yaml | Toml | Xml | Md | MdTable | Html | Txt | Rtf => std::str::from_utf8(b).is_ok(),
        other => panic!("no magic rule for {other:?}"),
    }
}

/// Parses CSV text into rows of owned fields (quotes/escapes resolved).
pub fn csv_rows(path: &Path) -> Vec<Vec<String>> {
    csv::ReaderBuilder::new()
        .has_headers(false)
        .from_path(path)
        .expect("open csv")
        .records()
        .map(|r| r.expect("csv record").iter().map(str::to_owned).collect())
        .collect()
}

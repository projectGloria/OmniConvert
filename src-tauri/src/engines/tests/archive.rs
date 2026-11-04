use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;

use zip::write::SimpleFileOptions;

use super::{ctx, spec, to};
use crate::engines::archive::convert;
use crate::engines::error::EngineError;
use crate::registry::{Format::*, TargetKind};

fn make_zip(path: &Path, files: &[(&str, &[u8])]) {
    let mut z = zip::ZipWriter::new(File::create(path).unwrap());
    for (name, data) in files {
        z.start_file(*name, SimpleFileOptions::default()).unwrap();
        z.write_all(data).unwrap();
    }
    z.finish().unwrap();
}

fn read_tree(root: &Path) -> BTreeMap<String, Vec<u8>> {
    let mut out = BTreeMap::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for e in fs::read_dir(dir).unwrap().flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else {
                let rel = p.strip_prefix(root).unwrap().to_string_lossy().replace('\\', "/");
                out.insert(rel, fs::read(&p).unwrap());
            }
        }
    }
    out
}

#[test]
fn zip_to_tgz_to_7z_to_txz_and_extract_preserves_everything() {
    let d = tempfile::tempdir().unwrap();
    let big: Vec<u8> = (0..300_000u32).map(|i| (i % 251) as u8).collect();
    let files: [(&str, &[u8]); 3] = [("readme.txt", b"hello"), ("docs/deep/a.bin", &big), ("docs/b.txt", b"")];
    let zip = d.path().join("in.zip");
    make_zip(&zip, &files);

    let chain = [(Zip, Tgz), (Tgz, SevenZ), (SevenZ, Txz), (Txz, Zip)];
    let mut current = zip;
    for (i, (src, dst)) in chain.into_iter().enumerate() {
        let next = d.path().join(format!("step{i}.{}", dst.ext()));
        convert(&spec(&current, src, to(dst), next.clone()), &ctx()).unwrap_or_else(|e| panic!("{src:?}→{dst:?}: {e}"));
        current = next;
    }

    let out = d.path().join("extracted");
    convert(&spec(&current, Zip, TargetKind::Extract, out.clone()), &ctx()).unwrap();
    let expected: BTreeMap<String, Vec<u8>> = files.iter().map(|(n, b)| ((*n).to_owned(), b.to_vec())).collect();
    assert_eq!(read_tree(&out), expected);
}

#[test]
fn zip_slip_entries_are_dropped_on_extract() {
    let d = tempfile::tempdir().unwrap();
    let zip = d.path().join("evil.zip");
    make_zip(&zip, &[("../escape.txt", b"pwned"), ("ok.txt", b"fine")]);
    let out = d.path().join("x");
    convert(&spec(&zip, Zip, TargetKind::Extract, out.clone()), &ctx()).unwrap();
    assert!(!d.path().join("escape.txt").exists());
    assert_eq!(read_tree(&out).keys().collect::<Vec<_>>(), ["ok.txt"]);
}

#[test]
fn gz_decompresses_to_a_single_file() {
    let d = tempfile::tempdir().unwrap();
    let gz = d.path().join("notes.txt.gz");
    let mut enc = flate2::write::GzEncoder::new(File::create(&gz).unwrap(), flate2::Compression::default());
    enc.write_all(b"plain text").unwrap();
    enc.finish().unwrap();

    let out = d.path().join("notes");
    convert(&spec(&gz, Gz, TargetKind::Extract, out.clone()), &ctx()).unwrap();
    assert_eq!(fs::read(out.join("notes.txt")).unwrap(), b"plain text");
}

#[test]
fn cancellation_stops_mid_archive() {
    let d = tempfile::tempdir().unwrap();
    let zip = d.path().join("in.zip");
    make_zip(&zip, &[("a.txt", b"a"), ("b.txt", b"b")]);
    let c = ctx();
    c.cancel.cancel();
    let err = convert(&spec(&zip, Zip, to(Tar), d.path().join("o.tar")), &c).unwrap_err();
    assert!(matches!(err, EngineError::Cancelled), "{err}");
}

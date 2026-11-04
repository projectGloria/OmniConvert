//! Inputs generated at test time into a TempDir: no binary fixtures in git, and every
//! test gets pristine files (outputs land next to inputs, like the app's default).

use std::fs;
use std::io::Write;
use std::path::PathBuf;

use image::{Rgba, RgbaImage};
use tempfile::TempDir;

pub struct Fixtures {
    // Held for its Drop: the directory (and every output written into it) is removed
    // when the test ends, pass or fail.
    _dir: TempDir,
    pub root: PathBuf,
}

pub const CSV: &str = "id,name,zip,active\n1,Ada,02134,true\n2,\"Lin, Grace\",90210,false\n3,Émile,,true\n";

pub const JSON: &str = r#"[
  { "id": 1, "name": "Ada", "score": 9.5, "tags": "x" },
  { "id": 2, "name": "Lin", "score": 7.25, "tags": "y" }
]"#;

pub const MD: &str = "# Quarterly Report\n\nIntro paragraph with *emphasis*.\n\n## Numbers\n\n\
| Region | Units |\n|---|---:|\n| North | 120 |\n| South | 95 |\n\n## Notes\n\n- one\n- two\n";

pub const PNG_SIDE: u32 = 64;
pub const WAV_RATE: u32 = 44_100;

impl Fixtures {
    pub fn new() -> Self {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path().to_path_buf();
        Self { _dir: dir, root }
    }

    fn write(&self, name: &str, body: &[u8]) -> PathBuf {
        let p = self.root.join(name);
        fs::write(&p, body).expect("write fixture");
        p
    }

    pub fn csv(&self) -> PathBuf {
        self.write("people.csv", CSV.as_bytes())
    }

    pub fn json(&self) -> PathBuf {
        self.write("scores.json", JSON.as_bytes())
    }

    pub fn md(&self) -> PathBuf {
        self.write("report.md", MD.as_bytes())
    }

    /// 64×64 RGBA gradient; the top-left quadrant is semi/fully transparent so alpha
    /// handling (WebP lossless, JPG flatten, ICO) is actually exercised.
    pub fn png(&self) -> (PathBuf, RgbaImage) {
        let img = RgbaImage::from_fn(PNG_SIDE, PNG_SIDE, |x, y| {
            let a = if x < 32 && y < 32 { ((x + y) * 4) as u8 } else { 255 };
            Rgba([(x * 4) as u8, (y * 4) as u8, 200, a])
        });
        let p = self.root.join("sprite.png");
        img.save(&p).expect("save png");
        (p, img)
    }

    /// 1 s, 440 Hz, mono 16-bit sine.
    pub fn wav(&self) -> (PathBuf, Vec<i16>) {
        let p = self.root.join("tone.wav");
        let spec = hound::WavSpec { channels: 1, sample_rate: WAV_RATE, bits_per_sample: 16, sample_format: hound::SampleFormat::Int };
        let samples: Vec<i16> = (0..WAV_RATE)
            .map(|i| ((i as f32 * 440.0 * std::f32::consts::TAU / WAV_RATE as f32).sin() * 0.5 * f32::from(i16::MAX)) as i16)
            .collect();
        let mut w = hound::WavWriter::create(&p, spec).expect("wav writer");
        samples.iter().for_each(|&s| w.write_sample(s).expect("sample"));
        w.finalize().expect("finalize wav");
        (p, samples)
    }

    /// ZIP with a nested folder, so extraction must recreate directories.
    pub fn zip(&self) -> PathBuf {
        let p = self.root.join("bundle.zip");
        let mut z = zip::ZipWriter::new(fs::File::create(&p).expect("zip file"));
        let opts = zip::write::SimpleFileOptions::default();
        for (name, body) in ARCHIVE_ENTRIES {
            z.start_file(*name, opts).expect("zip entry");
            z.write_all(body).expect("zip body");
        }
        z.finish().expect("finish zip");
        p
    }
}

pub const ARCHIVE_ENTRIES: &[(&str, &[u8])] = &[("readme.txt", b"hello archive\n"), ("data/nums.csv", b"a,b\n1,2\n")];

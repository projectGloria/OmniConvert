//! Document engine: pandoc, with typst as the PDF engine.

use super::error::{EngineError, EngineResult};
use super::util::path_arg as p;
use super::{ConvertSpec, JobCtx};
use crate::registry::{Format, TargetKind};
use crate::sidecar::{self, RunOptions};

fn reader(src: Format) -> EngineResult<&'static str> {
    Ok(match src {
        // Plain text is valid (if boring) Markdown — pandoc has no "txt" reader.
        Format::Md | Format::Txt => "markdown",
        Format::Html => "html",
        Format::Docx => "docx",
        Format::Rtf => "rtf",
        Format::Epub => "epub",
        other => return Err(EngineError::Unsupported(format!("pandoc can't read {}", other.label()))),
    })
}

fn writer(dst: Format) -> EngineResult<&'static str> {
    Ok(match dst {
        Format::Md => "gfm", // GitHub-flavored: tables + fenced code survive the round trip
        Format::Html => "html5",
        Format::Txt => "plain",
        Format::Docx => "docx",
        Format::Rtf => "rtf",
        Format::Epub => "epub3",
        Format::Pdf => "typst",
        other => return Err(EngineError::Unsupported(format!("pandoc can't write {}", other.label()))),
    })
}

pub async fn convert(spec: &ConvertSpec, ctx: &JobCtx) -> EngineResult<()> {
    let TargetKind::Convert(dst) = spec.target else {
        return Err(EngineError::Unsupported("not a document conversion".into()));
    };
    let pandoc = ctx.tools.pandoc()?;
    let input_dir = spec.input.parent().map(|d| d.to_path_buf());

    let mut args = vec![
        p(&spec.input),
        "-f".to_owned(), reader(spec.src)?.to_owned(),
        "-t".to_owned(), writer(dst)?.to_owned(),
        "-o".to_owned(), p(&spec.output),
    ];
    if let Some(dir) = &input_dir {
        // Relative image links in Markdown/HTML resolve against the source folder.
        args.extend(["--resource-path".to_owned(), p(dir)]);
    }
    if matches!(dst, Format::Html | Format::Rtf) {
        args.push("--standalone".to_owned()); // full document, not a body fragment
    }
    if dst == Format::Pdf {
        // Absolute path to the bundled typst; pandoc keys engine behaviour off the
        // basename, which is why dev builds use the plain-named binaries/dev/typst.exe.
        args.push(format!("--pdf-engine={}", p(ctx.tools.typst()?)));
    }
    if let Some(stem) = spec.input.file_stem() {
        // Silences pandoc's "no title" warning and gives HTML/EPUB a sensible <title>.
        args.push(format!("--metadata=pagetitle:{}", stem.to_string_lossy()));
    }

    ctx.progress.report(None, "Converting");
    sidecar::run(pandoc, &args, RunOptions { cwd: input_dir.as_deref(), cancel: &ctx.cancel }, |_| {}).await
}

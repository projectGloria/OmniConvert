//! OmniConvert web engine: a WASI command the browser worker runs once per request.
//!
//!   omniconvert-web probe   '{"paths":["/in/0/a.png"]}'                 → FileInfo[]
//!   omniconvert-web convert '{"input":…,"src":"png","target":…,"outDir":"/out","settings":{}}'
//!   omniconvert-web merge   '{"inputs":[["/in/0/a.png","png"]],"outDir":"/out","settings":{}}'
//!
//! stdout carries exactly one JSON reply (`{"output":"/out/a.webp"}` on success, named by
//! the same `output::plan` the desktop uses); stderr carries progress lines (see sink.rs).
//! Exit code 0 = ok, 1 = engine error (reply is `{"message","cancelled":false}`), 2 = bad call.
//! A fresh WASM instance runs each request, so cancellation is `worker.terminate()`:
//! the browser drops the whole instance, the same guarantee a Job Object gives the desktop.

mod caps;
mod sink;

use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;

use omniconvert_core::engines::error::{EngineError, EngineResult};
use omniconvert_core::engines::{archive, data, image, ConvertSpec, Ctx};
use omniconvert_core::output;
use omniconvert_core::registry::{self, route_for, EngineRoute, FileInfo, Format, TargetKind};
use omniconvert_core::settings::JobSettings;
use serde::Deserialize;
use serde_json::json;
use tokio_util::sync::CancellationToken;

#[derive(Deserialize)]
struct ProbeReq {
    paths: Vec<PathBuf>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ConvertReq {
    input: PathBuf,
    /// Supplied by the worker, not re-detected: for AVIF the worker has already decoded
    /// to PNG and passes `src: "png"` (the browser replaces the desktop's ffmpeg pass).
    src: Format,
    target: TargetKind,
    out_dir: PathBuf,
    #[serde(default)]
    settings: JobSettings,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct MergeReq {
    inputs: Vec<(PathBuf, Format)>,
    out_dir: PathBuf,
    #[serde(default)]
    settings: JobSettings,
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    let (Some(cmd), Some(body)) = (args.get(1), args.get(2)) else {
        return reply(2, &json!({ "message": "usage: omniconvert-web <probe|convert|merge> <json>" }));
    };
    let ctx = Ctx { cancel: CancellationToken::new(), progress: Arc::new(sink::StderrSink::default()) };

    let result = match cmd.as_str() {
        "probe" => parse::<ProbeReq>(body).map(|r| json!(probe(&r.paths))),
        "convert" => parse::<ConvertReq>(body).and_then(|r| convert(r, &ctx)).map(|out| json!({ "output": out })),
        "merge" => parse::<MergeReq>(body).and_then(|r| merge(r, &ctx)).map(|out| json!({ "output": out })),
        other => return reply(2, &json!({ "message": format!("unknown command {other}") })),
    };
    match result {
        Ok(v) => reply(0, &v),
        Err(e) => {
            let (message, detail) = e.into_parts();
            reply(1, &json!({ "message": message, "detail": detail }))
        }
    }
}

fn parse<T: for<'de> Deserialize<'de>>(body: &str) -> EngineResult<T> {
    serde_json::from_str(body).map_err(|e| EngineError::Unsupported(format!("bad request: {e}")))
}

/// Same detection + matrix as the desktop's `probe_files`, minus routes the browser can't run.
fn probe(paths: &[PathBuf]) -> Vec<FileInfo> {
    paths
        .iter()
        .filter_map(|p| fs::metadata(p).ok().filter(|m| m.is_file()).map(|m| (p, m.len())))
        .map(|(p, size)| {
            let mut info = registry::inspect(p, size);
            info.targets = caps::filter(info.format, std::mem::take(&mut info.targets));
            info
        })
        .collect()
}

/// Writes straight to the planned final path: the in-memory /out has no Explorer to
/// show a half-written file in, so the desktop's part-file + rename dance buys nothing.
fn convert(r: ConvertReq, ctx: &Ctx) -> EngineResult<PathBuf> {
    if matches!(r.target, TargetKind::Extract) {
        return Err(EngineError::Unsupported("extraction isn't available in the browser yet".into()));
    }
    let planned = output::plan(&r.input, r.src, r.target, Some(&r.out_dir), |_| false);
    let spec = ConvertSpec { input: r.input, src: r.src, target: r.target, output: planned.final_path, settings: r.settings };
    run(&spec, ctx)?;
    Ok(spec.output)
}

fn merge(r: MergeReq, ctx: &Ctx) -> EngineResult<PathBuf> {
    let first = r.inputs.first().ok_or_else(|| EngineError::Unsupported("nothing to merge".into()))?;
    let out = output::plan_merge(&first.0, Some(&r.out_dir), |_| false).final_path;
    image::merge::merge_pdf(&r.inputs, &r.settings, &out, ctx)?;
    Ok(out)
}

fn run(spec: &ConvertSpec, ctx: &Ctx) -> EngineResult<()> {
    match route_for(spec.src, spec.target) {
        EngineRoute::Image | EngineRoute::Svg => image::convert(spec, ctx),
        EngineRoute::Data => data::convert(spec, ctx),
        EngineRoute::Archive => archive::convert(spec, ctx),
        // The worker must decode AVIF itself and resend as PNG.
        EngineRoute::MediaThenImage => Err(EngineError::Unsupported("AVIF must be decoded by the browser first".into())),
        _ => Err(EngineError::Unsupported(format!("{} → {} needs the desktop app", spec.src.label(), spec.target.id()))),
    }
}

fn reply(code: u8, body: &serde_json::Value) -> ExitCode {
    println!("{body}");
    ExitCode::from(code)
}

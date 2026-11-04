//! Data & spreadsheet engine: read → `Doc` IR → write. N readers + M writers instead
//! of N×M converters (PLAN §2.4).

mod model;
mod read;
mod write;
mod xml;

use super::error::{EngineError, EngineResult};
use super::{ConvertSpec, Ctx};
use crate::registry::TargetKind;

pub fn convert(spec: &ConvertSpec, ctx: &Ctx) -> EngineResult<()> {
    let TargetKind::Convert(dst) = spec.target else {
        return Err(EngineError::Unsupported("not a data conversion".into()));
    };
    ctx.progress.report(None, "Reading");
    let doc = read::read(&spec.input, spec.src)?;
    ctx.check()?;
    ctx.progress.report(None, "Writing");
    write::write(doc, dst, &spec.output)
}

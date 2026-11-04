use std::path::Path;
use std::time::Duration;

use serde::Serialize;
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;
use ts_rs::TS;

use crate::sidecar::{self, RunOptions, Tools};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum HwEncoder {
    H264Nvenc,
    HevcNvenc,
    H264Amf,
    HevcAmf,
    H264Qsv,
    HevcQsv,
}

impl HwEncoder {
    pub const ALL: [HwEncoder; 6] =
        [Self::H264Nvenc, Self::HevcNvenc, Self::H264Amf, Self::HevcAmf, Self::H264Qsv, Self::HevcQsv];

    pub fn ffmpeg_name(self) -> &'static str {
        match self {
            Self::H264Nvenc => "h264_nvenc",
            Self::HevcNvenc => "hevc_nvenc",
            Self::H264Amf => "h264_amf",
            Self::HevcAmf => "hevc_amf",
            Self::H264Qsv => "h264_qsv",
            Self::HevcQsv => "hevc_qsv",
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct GpuCaps {
    /// False until the startup probe finishes (UI shows presets as "detecting…").
    pub ready: bool,
    pub encoders: Vec<HwEncoder>,
}

impl GpuCaps {
    pub fn has(&self, e: HwEncoder) -> bool {
        self.encoders.contains(&e)
    }
}

const PROBE_TIMEOUT: Duration = Duration::from_secs(10);

/// Probe every hardware encoder by *actually encoding one frame*.
///
/// Parsing `ffmpeg -encoders` is useless here: an NVENC-enabled build lists
/// `h264_nvenc` on every machine, GPU or not. Only a real encode proves the driver,
/// the hardware and the session limit all cooperate.
pub async fn detect(tools: &Tools) -> GpuCaps {
    let Some(ffmpeg) = tools.ffmpeg.clone() else {
        return GpuCaps { ready: true, encoders: vec![] };
    };

    let mut set = JoinSet::new();
    for enc in HwEncoder::ALL {
        let ffmpeg = ffmpeg.clone();
        set.spawn(async move { probe_one(&ffmpeg, enc).await.then_some(enc) });
    }

    let mut encoders: Vec<HwEncoder> = set.join_all().await.into_iter().flatten().collect();
    encoders.sort_by_key(|e| HwEncoder::ALL.iter().position(|x| x == e)); // stable priority order
    GpuCaps { ready: true, encoders }
}

async fn probe_one(ffmpeg: &Path, enc: HwEncoder) -> bool {
    let cancel = CancellationToken::new();
    let args = [
        "-hide_banner", "-v", "error", "-f", "lavfi", "-i", "color=c=black:s=256x256:d=0.1",
        "-frames:v", "1", "-c:v", enc.ffmpeg_name(), "-f", "null", "-",
    ];
    let run = sidecar::run(ffmpeg, args, RunOptions { cwd: None, cancel: &cancel }, |_| {});
    match tokio::time::timeout(PROBE_TIMEOUT, run).await {
        Ok(result) => result.is_ok(),
        Err(_) => {
            // A hung driver init: cancel kills the probe's process tree via its Job Object.
            cancel.cancel();
            false
        }
    }
}

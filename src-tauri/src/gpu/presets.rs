//! Preset × container × detected hardware → ffmpeg video encoder arguments.

use super::detect::{GpuCaps, HwEncoder};
use crate::queue::job::Preset;
use crate::registry::Format;

pub struct VideoEncode {
    pub args: Vec<String>,
    /// Which hardware encoder was chosen, if any (drives the GPU pool + CPU retry).
    pub hw: Option<HwEncoder>,
}

fn s(v: &[&str]) -> Vec<String> {
    v.iter().map(|x| (*x).to_owned()).collect()
}

pub fn video_encoder(container: Format, preset: Preset, caps: &GpuCaps) -> VideoEncode {
    if container == Format::Webm {
        // WebM ⇒ VP9. No consumer-GPU VP9 *encoder* is worth targeting here; always CPU.
        let (crf, deadline, cpu_used) = match preset {
            Preset::Fast => ("36", "realtime", "6"),
            Preset::Balanced => ("32", "good", "3"),
            Preset::Archival => ("28", "good", "1"),
        };
        let args = s(&["-c:v", "libvpx-vp9", "-crf", crf, "-b:v", "0", "-row-mt", "1", "-deadline", deadline, "-cpu-used", cpu_used]);
        return VideoEncode { args, hw: None };
    }

    if preset == Preset::Archival {
        // HEVC in AVI/FLV is unplayable in practice, so those stay H.264 even for archival.
        let args = if matches!(container, Format::Avi | Format::Flv) {
            s(&["-c:v", "libx264", "-preset", "slow", "-crf", "18"])
        } else {
            let mut a = s(&["-c:v", "libx265", "-preset", "slow", "-crf", "20"]);
            if matches!(container, Format::Mp4 | Format::Mov) {
                a.extend(s(&["-tag:v", "hvc1"])); // Apple players refuse the default hev1 tag
            }
            a
        };
        return VideoEncode { args: with_yuv420(args), hw: None };
    }

    let fast = preset == Preset::Fast;
    let hw = [HwEncoder::H264Nvenc, HwEncoder::H264Amf, HwEncoder::H264Qsv].into_iter().find(|e| caps.has(*e));
    let args = match hw {
        Some(HwEncoder::H264Nvenc) => {
            s(&["-c:v", "h264_nvenc", "-preset", if fast { "p2" } else { "p5" }, "-rc", "vbr", "-cq", if fast { "28" } else { "23" }, "-b:v", "0"])
        }
        Some(HwEncoder::H264Amf) => s(&[
            "-c:v", "h264_amf", "-quality", if fast { "speed" } else { "balanced" }, "-rc", "cqp",
            "-qp_i", if fast { "26" } else { "22" }, "-qp_p", if fast { "28" } else { "24" },
        ]),
        Some(HwEncoder::H264Qsv) => {
            s(&["-c:v", "h264_qsv", "-preset", if fast { "veryfast" } else { "medium" }, "-global_quality", if fast { "28" } else { "23" }])
        }
        _ => s(&["-c:v", "libx264", "-preset", if fast { "veryfast" } else { "medium" }, "-crf", if fast { "26" } else { "22" }]),
    };
    VideoEncode { args: with_yuv420(args), hw }
}

/// 4:2:0 8-bit is the only pixel format every player (and every browser) decodes.
/// Sources like screen recordings or ProRes are often 4:4:4 / 10-bit.
fn with_yuv420(mut args: Vec<String>) -> Vec<String> {
    args.extend(s(&["-pix_fmt", "yuv420p"]));
    args
}

/// Audio track codec for a video container.
pub fn video_audio_args(container: Format) -> Vec<String> {
    match container {
        Format::Webm => s(&["-c:a", "libopus", "-b:a", "128k"]),
        Format::Avi => s(&["-c:a", "libmp3lame", "-b:a", "192k"]),
        _ => s(&["-c:a", "aac", "-b:a", "192k"]),
    }
}

/// Codec args for an audio-only target. `kbps` only applies to lossy codecs.
pub fn audio_args(target: Format, kbps: Option<u32>) -> Vec<String> {
    let br = |default: u32| format!("{}k", kbps.unwrap_or(default));
    match target {
        Format::Mp3 => vec!["-c:a".into(), "libmp3lame".into(), "-b:a".into(), br(192)],
        Format::Aac | Format::M4a => vec!["-c:a".into(), "aac".into(), "-b:a".into(), br(192)],
        Format::Ogg => vec!["-c:a".into(), "libvorbis".into(), "-b:a".into(), br(192)],
        Format::Opus => vec!["-c:a".into(), "libopus".into(), "-b:a".into(), br(128)],
        Format::Flac => s(&["-c:a", "flac"]),
        _ => s(&["-c:a", "pcm_s16le"]), // WAV
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn archival_never_uses_gpu_and_fast_prefers_nvenc() {
        let caps = GpuCaps { ready: true, encoders: vec![HwEncoder::H264Amf, HwEncoder::H264Nvenc] };
        assert!(video_encoder(Format::Mp4, Preset::Archival, &caps).hw.is_none());
        assert_eq!(video_encoder(Format::Mp4, Preset::Fast, &caps).hw, Some(HwEncoder::H264Nvenc));
        let cpu = video_encoder(Format::Mp4, Preset::Balanced, &GpuCaps::default());
        assert!(cpu.hw.is_none() && cpu.args.contains(&"libx264".to_owned()));
    }
}

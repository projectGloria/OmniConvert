//! ffmpeg argument construction per target kind.

use crate::engines::error::{EngineError, EngineResult};
use crate::engines::util::path_arg;
use crate::engines::ConvertSpec;
use crate::gpu::{presets, GpuCaps};
use crate::registry::{Category, Format, TargetKind};

pub struct Plan {
    pub args: Vec<String>,
    /// A hardware encoder was selected ⇒ GPU pool + one CPU retry on failure.
    pub uses_gpu: bool,
}

const DEFAULT_ANIM_FPS: u32 = 15;
const DEFAULT_ANIM_WIDTH: u32 = 480;

fn s(v: &[&str]) -> Vec<String> {
    v.iter().map(|x| (*x).to_owned()).collect()
}

/// Common prefix: machine-readable progress on stdout (`-progress pipe:1`), quiet
/// stderr except real errors, never prompt (`-nostdin`; a prompt would hang forever
/// with no console attached), overwrite the part file (`-y`).
fn prefix(input: &str) -> Vec<String> {
    s(&["-hide_banner", "-nostdin", "-y", "-v", "error", "-nostats", "-progress", "pipe:1", "-i", input])
}

pub fn build(spec: &ConvertSpec, caps: &GpuCaps) -> EngineResult<Plan> {
    let input = path_arg(&spec.input);
    let output = path_arg(&spec.output);
    let st = &spec.settings;
    let mut args = prefix(&input);
    let mut uses_gpu = false;

    match spec.target {
        TargetKind::Convert(dst) if dst.category() == Category::Audio => {
            // -vn: drop video; -map_metadata: keep title/artist tags.
            args.extend(s(&["-vn", "-sn", "-map_metadata", "0"]));
            args.extend(presets::audio_args(dst, st.audio_bitrate));
        }
        TargetKind::Convert(dst) if dst.category() == Category::Video => {
            let enc = presets::video_encoder(dst, st.preset, caps);
            uses_gpu = enc.hw.is_some();
            // yuv420p needs even dimensions; scale=-2:H keeps aspect AND evenness.
            let scale = match st.video_height {
                Some(h) => format!("scale=-2:{h}"),
                None => "scale=trunc(iw/2)*2:trunc(ih/2)*2".to_owned(),
            };
            args.extend(s(&["-vf", &scale, "-sn"])); // subtitles: most containers can't take most sub codecs
            args.extend(enc.args);
            if spec.src != Format::Gif {
                args.extend(presets::video_audio_args(dst));
            }
            if matches!(dst, Format::Mp4 | Format::Mov) {
                args.extend(s(&["-movflags", "+faststart"])); // moov atom first ⇒ streams/previews instantly
            }
        }
        TargetKind::AnimatedGif => {
            let (fps, width) = (st.fps.unwrap_or(DEFAULT_ANIM_FPS), st.max_dimension.unwrap_or(DEFAULT_ANIM_WIDTH));
            // Two-pass palette in one graph: a per-clip 256-colour palette instead of
            // the generic web palette — the difference between banding and clean GIFs.
            let graph = format!(
                "fps={fps},scale='min({width},iw)':-1:flags=lanczos,split[a][b];\
                 [a]palettegen=stats_mode=diff[p];[b][p]paletteuse=dither=bayer:bayer_scale=5"
            );
            args.extend(s(&["-vf", &graph, "-an", "-loop", "0"]));
        }
        TargetKind::AnimatedWebp => {
            let (fps, width) = (st.fps.unwrap_or(DEFAULT_ANIM_FPS), st.max_dimension.unwrap_or(DEFAULT_ANIM_WIDTH));
            let q = st.quality.unwrap_or(75).to_string();
            let vf = format!("fps={fps},scale='min({width},iw)':-1:flags=lanczos");
            args.extend(s(&["-vf", &vf, "-an", "-c:v", "libwebp_anim", "-loop", "0", "-q:v", &q]));
        }
        other => return Err(EngineError::Unsupported(format!("media engine can't produce {}", other.id()))),
    }

    args.push(output);
    Ok(Plan { args, uses_gpu })
}

/// Frame extraction into `dir/frame_00001.png`. Default 1 fps — every frame of a
/// 10-minute 30fps clip would be 18,000 PNGs, which is never what anyone wants.
pub fn frames(spec: &ConvertSpec, dir: &std::path::Path) -> Vec<String> {
    let fps = spec.settings.fps.unwrap_or(1).to_string();
    let mut args = prefix(&path_arg(&spec.input));
    args.extend(s(&["-vf", &format!("fps={fps}"), "-an", "-sn"]));
    args.push(path_arg(&dir.join("frame_%05d.png")));
    args
}

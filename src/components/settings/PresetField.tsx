import { useApp } from "../../store/appStore";
import type { Format } from "../../types/generated/Format";
import type { GpuCaps } from "../../types/generated/GpuCaps";
import type { Preset } from "../../types/generated/Preset";
import { Field, Segmented, type Option } from "./controls";

const PRESETS: readonly Option<Preset>[] = [
  { value: "fast", label: "Fast", title: "GPU constant quality, speed-biased" },
  { value: "balanced", label: "Balanced", title: "GPU constant quality, quality-biased" },
  { value: "archival", label: "Archival", title: "CPU x265 CRF — smallest files, slowest" },
];

/** Mirrors gpu::presets::video_encoder's choice so the chip never lies about what runs. */
function encoderLabel(gpu: GpuCaps, preset: Preset, container: Format | null): string {
  if (container === "webm") return "CPU · VP9"; // no GPU VP9 encode path
  if (preset === "archival") return container === "avi" || container === "flv" ? "CPU · x264" : "CPU · x265";
  if (!gpu.ready) return "detecting GPU…";
  // Same preference order as Rust: NVENC → AMF → QSV.
  if (gpu.encoders.includes("h264Nvenc")) return "GPU · NVENC";
  if (gpu.encoders.includes("h264Amf")) return "GPU · AMF";
  if (gpu.encoders.includes("h264Qsv")) return "GPU · QSV";
  return "CPU · x264";
}

export function PresetField({ preset, container, onChange }: { preset: Preset; container: Format | null; onChange: (p: Preset) => void }) {
  const gpu = useApp((s) => s.gpu);
  const label = encoderLabel(gpu, preset, container);
  const cpu = label.startsWith("CPU");
  // Deliberate deviation from "disable GPU presets without a GPU": Rust already falls back
  // to x264 veryfast/medium, and forcing Archival (x265 slow) on GPU-less machines would
  // turn a 1-minute job into a 20-minute one. We say what will run instead.
  const why = cpu && gpu.ready && gpu.encoders.length === 0 && preset !== "archival" ? "No GPU encoder found — runs on CPU" : undefined;

  return (
    <Field
      label="Preset"
      hint={
        <span
          title={why}
          className={`rounded px-1.5 py-px font-mono text-[10px] ${cpu ? "bg-raised text-muted" : "bg-success/10 text-success"}`}
        >
          {label}
        </span>
      }
    >
      <Segmented name="preset" options={PRESETS} value={preset} onChange={(p) => onChange(p ?? "balanced")} />
    </Field>
  );
}

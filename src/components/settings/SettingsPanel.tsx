import { RotateCcw } from "lucide-react";
import { FIELD_KEY, type FieldSpec, type NumericKey } from "../../lib/settingsSchema";
import { useQueue } from "../../store/queueStore";
import type { Target } from "../../types/generated/Target";
import type { QueueItem } from "../../types/queue";
import { Field, NumberInput, Segmented, Slider, type Option } from "./controls";
import { PresetField } from "./PresetField";

const opts = (values: readonly (number | null)[], fmt: (n: number) => string): Option<number>[] =>
  values.map((v) => ({ value: v, label: v === null ? "Original" : fmt(v) }));

const HEIGHTS = opts([null, 1080, 720, 480], (n) => `${n}p`);
const ANIM_FPS = opts([10, 15, 24, 30], String);
const FRAME_FPS = opts([1, 2, 5, 10], (n) => `${n}/s`);
const ANIM_WIDTH = opts([320, 480, 720, 1080], (n) => `${n}px`);
const BITRATES = opts([96, 128, 192, 256, 320], String);
const RATES = opts([null, 44100, 48000], (n) => `${n / 1000}k`);
const EDGES = opts([null, 3840, 1920, 1280, 640], String);
const DPIS = opts([72, 96, 150, 300], String);

interface Props {
  item: QueueItem;
  target: Target | undefined;
  fields: readonly FieldSpec[];
}

/** Renders exactly the fields settingsSchema says apply. No field ⇒ never mounted. */
export function SettingsPanel({ item, target, fields }: Props) {
  const setSettings = useQueue((s) => s.setSettings);
  const st = item.settings;

  // Sparse diff: choosing the engine default *removes* the key rather than pinning it,
  // so a future default change in Rust reaches rows the user never really customized.
  const setNum = (key: NumericKey, fallback: FieldSpec["fallback"]) => (v: number | null) =>
    setSettings(item.id, { [key]: v === null || v === fallback ? undefined : v });
  const numOf = (key: NumericKey, fallback: FieldSpec["fallback"]) => st[key] ?? (typeof fallback === "number" ? fallback : null);

  const reset = () => setSettings(item.id, Object.fromEntries(Object.keys(st).map((k) => [k, undefined])));
  const dst = target?.kind.type === "convert" ? target.kind.format : null;

  return (
    <div className="w-72 space-y-3.5 rounded-lg border border-line bg-surface p-3.5 shadow-2xl shadow-black/60">
      <div className="flex items-center justify-between">
        <h2 className="text-xs font-semibold text-ink">{target?.label ?? "Settings"}</h2>
        <button type="button" onClick={reset} className="flex items-center gap-1 text-[11px] text-muted hover:text-ink">
          <RotateCcw size={11} /> Defaults
        </button>
      </div>

      {fields.map(({ field, fallback }) => {
        // Every field past the two non-numeric cases below writes a numeric key.
        const key = FIELD_KEY[field] as NumericKey;
        switch (field) {
          case "preset":
            return <PresetField key={field} preset={st.preset} container={dst} onChange={(p) => setSettings(item.id, { preset: p })} />;
          case "background":
            return (
              <Field key={field} label="Transparent areas become" hint={<span className="font-mono text-[10px] text-muted">{st.background ?? "#ffffff"}</span>}>
                <input
                  type="color"
                  value={st.background ?? String(fallback)}
                  onChange={(e) => setSettings(item.id, { background: e.target.value === fallback ? undefined : e.target.value })}
                  className="h-7 w-full cursor-pointer rounded-md border border-line bg-base"
                />
              </Field>
            );
          case "quality": {
            const q = numOf("quality", fallback) ?? 85;
            const lossless = dst === "webp" && q >= 100;
            return (
              <Field key={field} label="Quality" hint={<span className="text-[11px] text-ink tabular-nums">{lossless ? "Lossless" : q}</span>}>
                <Slider value={q} onChange={setNum("quality", fallback)} />
              </Field>
            );
          }
          case "maxDimension":
            return (
              <Field key={field} label="Resize long edge" hint={<NumberInput value={st.maxDimension ?? null} onChange={setNum(key, fallback)} placeholder="custom" suffix="px" />}>
                <Segmented name={`${item.id}-${field}`} options={EDGES} value={st.maxDimension ?? null} onChange={setNum(key, fallback)} />
              </Field>
            );
        }
        // Remaining fields are all "pick one of N numbers".
        const [label, options] = {
          videoHeight: ["Resolution", HEIGHTS],
          fps: ["Frame rate", fallback === 1 ? FRAME_FPS : ANIM_FPS],
          animWidth: ["Max width", ANIM_WIDTH],
          audioBitrate: ["Bitrate (kbps)", BITRATES],
          sampleRate: ["Sample rate", RATES],
          dpi: ["DPI", DPIS],
        }[field] as [string, Option<number>[]];
        return (
          <Field key={field} label={label}>
            <Segmented name={`${item.id}-${field}`} options={options} value={numOf(key, fallback)} onChange={setNum(key, fallback)} />
          </Field>
        );
      })}
    </div>
  );
}

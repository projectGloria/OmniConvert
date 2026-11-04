import type { Format } from "../types/generated/Format";
import type { JobSettings } from "../types/generated/JobSettings";
import type { Target } from "../types/generated/Target";
import type { QueueItem } from "../types/queue";

/**
 * Which knobs apply to (source, target) and what Rust will use when a knob is unset.
 *
 * Single source for three things that would otherwise drift apart: which controls the
 * gear drawer renders, the placeholder/default each control shows, and which keys
 * survive a target change (pruneSettings). Defaults MUST mirror the Rust engines —
 * they're display-only; the frontend never sends a default, it sends nothing.
 */
export type Field =
  | "preset"
  | "videoHeight"
  | "fps"
  | "animWidth"
  | "quality"
  | "audioBitrate"
  | "sampleRate"
  | "maxDimension"
  | "background"
  | "dpi";

/** Settings key each field writes. animWidth and maxDimension share one wire key. */
export const FIELD_KEY = {
  preset: "preset",
  videoHeight: "videoHeight",
  fps: "fps",
  animWidth: "maxDimension",
  quality: "quality",
  audioBitrate: "audioBitrate",
  sampleRate: "sampleRate",
  maxDimension: "maxDimension",
  background: "background",
  dpi: "dpi",
} as const satisfies Record<Field, keyof JobSettings>;

export type NumericKey = "videoHeight" | "fps" | "maxDimension" | "quality" | "audioBitrate" | "sampleRate" | "dpi";

export interface FieldSpec {
  field: Field;
  /** Engine default for this (source, target) — what "unset" means. null = keep source's. */
  fallback: number | string | null;
}

const LOSSY_AUDIO = new Set<Format>(["mp3", "aac", "m4a", "ogg", "opus"]);
const QUALITY_IMAGE = new Set<Format>(["jpg", "webp", "avif"]);
const AUDIO_KBPS: Partial<Record<Format, number>> = { opus: 128 };

export function fieldsFor(item: Pick<QueueItem, "format" | "merge">, target: Target | undefined): FieldSpec[] {
  if (item.merge) return [{ field: "maxDimension", fallback: null }];
  if (!target) return [];
  const kind = target.kind;

  switch (kind.type) {
    case "animatedGif":
      return [{ field: "fps", fallback: 15 }, { field: "animWidth", fallback: 480 }];
    case "animatedWebp":
      return [{ field: "fps", fallback: 15 }, { field: "animWidth", fallback: 480 }, { field: "quality", fallback: 75 }];
    case "framesZip":
      return [{ field: "fps", fallback: 1 }];
    case "extract":
      return [];
    case "convert":
      break;
  }

  const dst = kind.format;
  if (target.group === "video") return [{ field: "preset", fallback: "balanced" }, { field: "videoHeight", fallback: null }];
  if (target.group === "audio") {
    const out: FieldSpec[] = [];
    if (LOSSY_AUDIO.has(dst)) out.push({ field: "audioBitrate", fallback: AUDIO_KBPS[dst] ?? 192 });
    // libopus is 48 kHz-only; Rust ignores the field for it, so don't offer it.
    if (dst !== "opus") out.push({ field: "sampleRate", fallback: null });
    return out;
  }

  if (item.format === "pdf") {
    // PDF → page images. Rust: 150 DPI, JPEG q90.
    const out: FieldSpec[] = [{ field: "dpi", fallback: 150 }];
    if (dst === "jpg") out.push({ field: "quality", fallback: 90 });
    return out;
  }

  if (target.group === "image" || dst === "pdf") {
    const out: FieldSpec[] = [];
    if (QUALITY_IMAGE.has(dst)) out.push({ field: "quality", fallback: 85 });
    // ICO always renders its own fixed size ladder (16…256), so a resize is meaningless.
    if (dst !== "ico") out.push({ field: "maxDimension", fallback: null });
    if (item.format === "svg") out.push({ field: "dpi", fallback: 96 });
    if (dst === "jpg") out.push({ field: "background", fallback: "#ffffff" });
    return out;
  }
  return []; // data, documents, archives: nothing to tune
}

/**
 * Drops keys the new (source, target) doesn't use. Keeps `preset` (it's required on the
 * wire). This is what makes "change target" safe: no stale `quality: 40` from a JPG
 * choice silently riding along into an AVIF encode later.
 */
export function pruneSettings(settings: JobSettings, fields: readonly FieldSpec[]): JobSettings {
  const keep = new Set<keyof JobSettings>(fields.map((f) => FIELD_KEY[f.field]));
  const next: JobSettings = { preset: settings.preset };
  for (const key of Object.keys(settings) as (keyof JobSettings)[]) {
    if (key !== "preset" && keep.has(key) && settings[key] !== undefined) {
      // Same key on both sides, so the value's type matches by construction.
      (next as Record<string, unknown>)[key] = settings[key];
    }
  }
  return next;
}

/** True when the user changed anything that applies here — drives the gear's dot. */
export function hasOverrides(settings: JobSettings, fields: readonly FieldSpec[]): boolean {
  return fields.some(({ field }) =>
    field === "preset" ? settings.preset !== "balanced" : settings[FIELD_KEY[field]] !== undefined,
  );
}

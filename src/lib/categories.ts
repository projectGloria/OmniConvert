import type { Category } from "../types/generated/Category";
import type { Target } from "../types/generated/Target";

/**
 * Per-domain tint. Desaturated on purpose: these only tint badge text/borders so the
 * Obsidian palette (indigo accent, emerald success) stays the loudest thing on screen.
 */
export const CATEGORY_TINT: Record<Category, string> = {
  video: "#a78bfa",
  audio: "#38bdf8",
  image: "#fbbf24",
  document: "#f472b6",
  data: "#34d399",
  archive: "#94a3b8",
};

export const CATEGORY_LABEL: Record<Category, string> = {
  video: "Video",
  audio: "Audio",
  image: "Image",
  document: "Document",
  data: "Data",
  archive: "Archive",
};

export interface TargetGroup {
  group: Category;
  targets: Target[];
}

/** Buckets targets by `group`, preserving the order Rust emitted them in. */
export function groupTargets(targets: readonly Target[]): TargetGroup[] {
  const groups: TargetGroup[] = [];
  for (const t of targets) {
    // Linear find is fine: at most 6 groups. A Map would buy nothing but noise.
    const existing = groups.find((g) => g.group === t.group);
    if (existing) existing.targets.push(t);
    else groups.push({ group: t.group, targets: [t] });
  }
  return groups;
}

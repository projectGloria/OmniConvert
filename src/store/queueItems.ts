import type { FileInfo } from "../types/generated/FileInfo";
import type { Target } from "../types/generated/Target";
import { MERGE_TARGET_ID, type JobId, type QueueItem } from "../types/queue";

// Pure item constructors, kept out of the store file so queueStore stays about state
// transitions rather than object literals.

export function fileItem(file: FileInfo, id: JobId): QueueItem {
  return {
    ...file,
    id,
    target: null,
    status: "idle",
    settings: { preset: "balanced" },
    output: null,
    outputBytes: null,
    error: null,
    errorDetail: null,
    finishedAt: null,
    merge: null,
  };
}

const MERGE_TARGET: Target = {
  id: MERGE_TARGET_ID,
  kind: { type: "convert", format: "pdf" },
  label: "PDF (one page per image)",
  group: "document",
  note: null,
};

/** A synthetic row for "Merge to PDF". Sources are snapshotted: later edits to the
 *  original rows (or removing them) don't change what this job will merge. */
export function mergeItem(sources: readonly QueueItem[], id: JobId): QueueItem {
  const [first] = sources;
  return {
    ...fileItem(
      {
        // Unique, non-filesystem path: never collides with the case-insensitive dedupe.
        path: `merge:${id}`,
        name: `${first?.name ?? "images"} + ${sources.length - 1} more → PDF`,
        ext: `${sources.length} img`,
        size: sources.reduce((n, s) => n + s.size, 0),
        format: null,
        category: "image",
        detectedAs: null,
        targets: [MERGE_TARGET],
      },
      id,
    ),
    target: MERGE_TARGET_ID, // the only possible target — preselected
    merge: sources.map((s) => s.path),
  };
}

/** Formats the Rust merge engine decodes natively (mirrors engines::image::merge::MERGEABLE). */
export const MERGEABLE = new Set(["png", "jpg", "webp", "bmp", "tiff", "gif", "ico", "svg"]);

export const isMergeable = (it: QueueItem | undefined): it is QueueItem =>
  !!it && !it.merge && it.format !== null && MERGEABLE.has(it.format);

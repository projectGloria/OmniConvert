import type { FileInfo } from "./generated/FileInfo";
import type { JobSettings } from "./generated/JobSettings";

// Wire types (FileInfo, Target, JobRequest, events…) are generated from Rust by ts-rs —
// see src/types/generated/ and `npm run bindings`. Only UI-local state lives here.

export type JobId = string;

export type ItemStatus = "idle" | "queued" | "running" | "done" | "error" | "cancelled";

/** Statuses from which "Convert" may (re)start an item. */
export const STARTABLE: ReadonlySet<ItemStatus> = new Set(["idle", "error", "cancelled"]);
/** Statuses with a live backend job (Cancel applies, editing is locked). */
export const ACTIVE: ReadonlySet<ItemStatus> = new Set(["queued", "running"]);
/** Terminal statuses: a batch is finished when every member is in one of these. */
export const FINAL: ReadonlySet<ItemStatus> = new Set(["done", "error", "cancelled"]);

export interface QueueItem extends FileInfo {
  id: JobId;
  /** Selected `Target.id`; null until the user picks one. */
  target: string | null;
  status: ItemStatus;
  /** Sparse diff over engine defaults (see lib/settingsSchema): unset key ⇒ Rust default. */
  settings: JobSettings;
  /** Final output path once done. */
  output: string | null;
  /** Bytes written (summed for folder outputs), for the batch summary. */
  outputBytes: number | null;
  error: string | null;
  /** Sidecar stderr tail behind the "details" disclosure. */
  errorDetail: string | null;
  /** Date.now() when the row reached a FINAL status — batch wall-clock time. */
  finishedAt: number | null;
  /** "Merge to PDF" rows: ordered source image paths. null for normal rows. */
  merge: string[] | null;
}

/** The one target a merge row offers. Not from the Rust matrix: merge is its own command. */
export const MERGE_TARGET_ID = "merge-pdf";

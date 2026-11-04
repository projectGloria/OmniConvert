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

export interface QueueItem extends FileInfo {
  id: JobId;
  /** Selected `Target.id`; null until the user picks one. */
  target: string | null;
  status: ItemStatus;
  /** Sparse per-item settings; Rust fills the defaults (Phase 4 drawer edits this). */
  settings: JobSettings;
  /** Final output path once done. */
  output: string | null;
  error: string | null;
}

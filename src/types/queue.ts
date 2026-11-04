import type { FileInfo } from "./generated/FileInfo";

// Wire types (FileInfo, Target, Format, Category…) are generated from Rust by ts-rs —
// see src/types/generated/ and `npm run bindings`. Only UI-local state lives here.

export type JobId = string;

export type ItemStatus = "idle" | "queued" | "running" | "done" | "error" | "cancelled";

export interface QueueItem extends FileInfo {
  id: JobId;
  /** Selected `Target.id`; null until the user picks one. */
  target: string | null;
  status: ItemStatus;
}

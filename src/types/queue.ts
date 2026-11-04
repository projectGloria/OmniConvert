// Phase 1 hand-written mirror of the Rust `ProbedFile` struct.
// Phase 2 replaces this with ts-rs generated bindings in src/types/generated/.

export type JobId = string;

/** Shape returned by the `probe_files` Tauri command (serde camelCase). */
export interface ProbedFile {
  path: string;
  name: string;
  ext: string;
  size: number;
}

export type ItemStatus = "idle" | "queued" | "running" | "done" | "error" | "cancelled";

export interface QueueItem extends ProbedFile {
  id: JobId;
  /** Target format id; null until the user picks one (Phase 2 registry). */
  target: string | null;
  status: ItemStatus;
}

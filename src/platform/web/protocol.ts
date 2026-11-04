import type { Format } from "../../types/generated/Format";
import type { JobSettings } from "../../types/generated/JobSettings";
import type { TargetKind } from "../../types/generated/TargetKind";

/** A File mounted read-only at `/in/<dir>/<file.name>` inside the WASI filesystem. */
export interface Mount {
  dir: string;
  file: File;
}

// The compiled WebAssembly.Module rides along with every request: compiled once on the
// main thread, structured-cloned to each worker for free (no re-download, no re-compile).
export type WorkerRequest =
  | { kind: "probe"; module: WebAssembly.Module; mounts: Mount[] }
  | { kind: "convert"; module: WebAssembly.Module; mount: Mount; src: Format; target: TargetKind; settings: JobSettings }
  | { kind: "merge"; module: WebAssembly.Module; mounts: Mount[]; formats: Format[]; settings: JobSettings };

export type WorkerReply =
  | { type: "progress"; progress: number | null; stage: string }
  | { type: "probed"; files: unknown[] }
  | { type: "done"; name: string; blob: Blob }
  | { type: "error"; message: string; detail: string | null };

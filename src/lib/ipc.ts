import { invoke } from "@tauri-apps/api/core";
import type { ProbedFile } from "../types/queue";

// Typed wrappers around invoke(). Components never call invoke() with raw strings,
// so a renamed Rust command breaks here — in one place — instead of at runtime.

/** Stats dropped paths in Rust; directories are expanded recursively. */
export const probeFiles = (paths: string[]) => invoke<ProbedFile[]>("probe_files", { paths });

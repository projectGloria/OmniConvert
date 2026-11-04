import { invoke } from "@tauri-apps/api/core";
import type { FileInfo } from "../types/generated/FileInfo";

// Typed wrappers around invoke(). Components never call invoke() with raw strings,
// so a renamed Rust command breaks here — in one place — instead of at runtime.

/** Detects formats and resolves valid targets in Rust; directories are expanded recursively. */
export const probeFiles = (paths: string[]) => invoke<FileInfo[]>("probe_files", { paths });

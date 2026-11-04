import { invoke } from "@tauri-apps/api/core";
import type { FileInfo } from "../types/generated/FileInfo";
import type { GpuCaps } from "../types/generated/GpuCaps";
import type { JobRequest } from "../types/generated/JobRequest";
import type { ToolStatus } from "../types/generated/ToolStatus";

// Typed wrappers around invoke(). Components never call invoke() with raw strings,
// so a renamed Rust command breaks here — in one place — instead of at runtime.

/** Detects formats and resolves valid targets in Rust; directories are expanded recursively. */
export const probeFiles = (paths: string[]) => invoke<FileInfo[]>("probe_files", { paths });

/** Fire-and-forget; results arrive as conversion://progress|complete|error events. */
export const enqueueJobs = (jobs: JobRequest[]) => invoke<void>("enqueue_jobs", { jobs });

export const cancelJob = (jobId: string) => invoke<void>("cancel_job", { jobId });

export const cancelAll = () => invoke<void>("cancel_all");

export const gpuCaps = () => invoke<GpuCaps>("gpu_caps");

export const toolStatus = () => invoke<ToolStatus>("tool_status");

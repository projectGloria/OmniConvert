import { platform } from "@platform";
import type { JobRequest } from "../types/generated/JobRequest";
import type { MergeRequest } from "../types/generated/MergeRequest";

// Typed host calls. Components never touch Tauri (or the web worker pool) directly:
// both hosts implement platform/types.ts, and `@platform` picks one at build time.

/** Detects formats and resolves valid targets in Rust; directories are expanded recursively. */
export const probeFiles = (paths: string[]) => platform.probeFiles(paths);

/** Fire-and-forget; results arrive as conversion://progress|complete|error events. */
export const enqueueJobs = (jobs: JobRequest[]) => platform.enqueueJobs(jobs);

/** One job, one output PDF; same conversion://* event contract, keyed by req.jobId. */
export const enqueueMerge = (req: MergeRequest) => platform.enqueueMerge(req);

export const cancelJob = (jobId: string) => platform.cancelJob(jobId);

export const cancelAll = () => platform.cancelAll();

export const gpuCaps = () => platform.gpuCaps();

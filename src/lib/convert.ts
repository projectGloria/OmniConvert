import type { JobRequest } from "../types/generated/JobRequest";
import type { MergeRequest } from "../types/generated/MergeRequest";
import { STARTABLE, type JobId } from "../types/queue";
import { useApp } from "../store/appStore";
import { useProgress } from "../store/progressStore";
import { useQueue } from "../store/queueStore";
import { enqueueJobs, enqueueMerge } from "./ipc";

/**
 * Starts startable rows that have a target: all of them, or just `only` (Retry).
 * Reads stores via getState() — this is an event handler, not render code, so
 * subscribing would only cause extra renders.
 */
export async function convert(only?: readonly JobId[]): Promise<void> {
  const { items, order, setStatus, startBatch } = useQueue.getState();
  const outputDir = useApp.getState().outputDir;
  const { setProgress } = useProgress.getState();

  const jobs: JobRequest[] = [];
  const merges: MergeRequest[] = [];
  for (const id of only ?? order) {
    const item = items[id];
    if (!item || !STARTABLE.has(item.status)) continue;
    const target = item.targets.find((t) => t.id === item.target);
    if (!target) continue;

    if (item.merge) merges.push({ jobId: id, paths: item.merge, outputDir, settings: item.settings });
    else jobs.push({ jobId: id, path: item.path, target: target.kind, outputDir, settings: item.settings });
    setStatus(id, "queued");
    setProgress(id, { value: 0, stage: "Queued" });
  }
  const started = [...jobs.map((j) => j.jobId), ...merges.map((m) => m.jobId)];
  if (started.length === 0) return;
  startBatch(started);

  // allSettled: an IPC failure on one call mustn't strand the other call's rows in "queued".
  const results = await Promise.allSettled([
    jobs.length ? enqueueJobs(jobs) : Promise.resolve(),
    ...merges.map((m) => enqueueMerge(m)),
  ]);
  const failed = (ids: string[], r: PromiseSettledResult<void> | undefined) => {
    // IPC itself failed (not a conversion error — those arrive as events).
    if (r?.status === "rejected") for (const id of ids) setStatus(id, "error", { error: String(r.reason) });
  };
  failed(jobs.map((j) => j.jobId), results[0]);
  merges.forEach((m, i) => failed([m.jobId], results[i + 1]));
}

import type { JobRequest } from "../types/generated/JobRequest";
import { STARTABLE } from "../types/queue";
import { useApp } from "../store/appStore";
import { useProgress } from "../store/progressStore";
import { useQueue } from "../store/queueStore";
import { enqueueJobs } from "./ipc";

/**
 * Starts every startable row that has a target. Reads stores via getState() — this is
 * an event handler, not render code, so subscribing would only cause extra renders.
 */
export async function convertAll(): Promise<void> {
  const { items, order, setStatus } = useQueue.getState();
  const outputDir = useApp.getState().outputDir;
  const { setProgress } = useProgress.getState();

  const jobs: JobRequest[] = [];
  for (const id of order) {
    const item = items[id];
    if (!item || !STARTABLE.has(item.status)) continue;
    const target = item.targets.find((t) => t.id === item.target);
    if (!target) continue;

    jobs.push({ jobId: id, path: item.path, target: target.kind, outputDir, settings: item.settings });
    setStatus(id, "queued");
    setProgress(id, { value: 0, stage: "Queued" });
  }
  if (jobs.length === 0) return;

  try {
    await enqueueJobs(jobs);
  } catch (err) {
    // IPC itself failed (not a conversion error — those arrive as events).
    for (const j of jobs) setStatus(j.jobId, "error", { error: String(err) });
  }
}

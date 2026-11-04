import { create } from "zustand";
import type { JobId } from "../types/queue";

export interface JobProgress {
  /** 0..1, or null while the engine can't measure (pandoc, tar streams) → indeterminate bar. */
  value: number | null;
  stage: string;
}

/**
 * Hot-path store, deliberately split from queueStore.
 *
 * Progress events arrive ~10×/s per running job. If progress lived on QueueItem,
 * each tick would replace the item object → re-render the whole row (badge, pill,
 * menus). Here only <ProgressBar>/<RowStatus> subscribed to `progress[id]` re-render.
 */
interface ProgressState {
  progress: Record<JobId, JobProgress>;
  setProgress: (id: JobId, p: JobProgress) => void;
  drop: (id: JobId) => void;
  reset: () => void;
}

export const useProgress = create<ProgressState>()((set) => ({
  progress: {},
  setProgress: (id, p) =>
    set((s) => ({
      progress: { ...s.progress, [id]: { value: p.value === null ? null : Math.min(1, Math.max(0, p.value)), stage: p.stage } },
    })),
  drop: (id) =>
    set((s) => {
      if (!(id in s.progress)) return s; // returning the same state = no notify
      const { [id]: _gone, ...progress } = s.progress;
      return { progress };
    }),
  reset: () => set({ progress: {} }),
}));

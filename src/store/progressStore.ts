import { create } from "zustand";
import type { JobId } from "../types/queue";

/**
 * Hot-path store, deliberately split from queueStore.
 *
 * Progress events arrive ~10×/s per running job. If progress lived on QueueItem,
 * each tick would replace the item object → re-render the whole row (badge, pill,
 * menus). Here only the <ProgressBar id> subscribed to `progress[id]` re-renders.
 */
interface ProgressState {
  progress: Record<JobId, number>; // 0..1
  setProgress: (id: JobId, value: number) => void;
  drop: (id: JobId) => void;
  reset: () => void;
}

export const useProgress = create<ProgressState>()((set) => ({
  progress: {},
  setProgress: (id, value) =>
    set((s) => ({ progress: { ...s.progress, [id]: Math.min(1, Math.max(0, value)) } })),
  drop: (id) =>
    set((s) => {
      if (!(id in s.progress)) return s; // returning the same state = no notify
      const { [id]: _gone, ...progress } = s.progress;
      return { progress };
    }),
  reset: () => set({ progress: {} }),
}));

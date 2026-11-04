import { create } from "zustand";
import type { JobId, ProbedFile, QueueItem } from "../types/queue";
import { useProgress } from "./progressStore";

interface QueueState {
  /** Normalized map: a row update replaces one object, so only that row's selector fires. */
  items: Record<JobId, QueueItem>;
  /** Display order kept separately — reordering never touches item objects. */
  order: JobId[];
  /** Returns how many files were actually added (duplicates by path are skipped). */
  addFiles: (files: ProbedFile[]) => number;
  remove: (id: JobId) => void;
  clear: () => void;
}

export const useQueue = create<QueueState>()((set, get) => ({
  items: {},
  order: [],

  addFiles: (files) => {
    const { items, order } = get();
    // Windows paths are case-insensitive: C:\A.png and c:\a.png are the same file.
    const known = new Set(Object.values(items).map((i) => i.path.toLowerCase()));

    const nextItems = { ...items };
    const nextOrder = [...order];
    for (const file of files) {
      const key = file.path.toLowerCase();
      if (known.has(key)) continue;
      known.add(key); // also dedupes within the same drop (folder + file inside it)

      const id = crypto.randomUUID();
      nextItems[id] = { ...file, id, target: null, status: "idle" };
      nextOrder.push(id);
    }

    const added = nextOrder.length - order.length;
    // Skip the set() entirely on a no-op drop so subscribers don't get a new `order` ref.
    if (added > 0) set({ items: nextItems, order: nextOrder });
    return added;
  },

  remove: (id) => {
    set((s) => {
      const { [id]: _removed, ...items } = s.items;
      return { items, order: s.order.filter((x) => x !== id) };
    });
    useProgress.getState().drop(id); // keep the hot-path store from leaking dead entries
  },

  clear: () => {
    set({ items: {}, order: [] });
    useProgress.getState().reset();
  },
}));

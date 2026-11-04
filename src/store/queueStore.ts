import { create } from "zustand";
import type { FileInfo } from "../types/generated/FileInfo";
import { ACTIVE, type ItemStatus, type JobId, type QueueItem } from "../types/queue";
import { useProgress } from "./progressStore";

interface QueueState {
  /** Normalized map: a row update replaces one object, so only that row's selector fires. */
  items: Record<JobId, QueueItem>;
  /** Display order kept separately — reordering never touches item objects. */
  order: JobId[];
  /** Returns how many files were actually added (duplicates by path are skipped). */
  addFiles: (files: FileInfo[]) => number;
  /** Pass null to clear. Ignores ids Rust didn't offer for this file. */
  setTarget: (id: JobId, targetId: string | null) => void;
  /** Lifecycle transitions driven by backend events (and by Convert). */
  setStatus: (id: JobId, status: ItemStatus, extra?: { output?: string; error?: string }) => void;
  remove: (id: JobId) => void;
  clear: () => void;
}

/** Immutable single-item patch; returns the same state object on no-op so nothing re-renders. */
function patch(s: QueueState, id: JobId, fn: (item: QueueItem) => QueueItem | null): QueueState | Partial<QueueState> {
  const item = s.items[id];
  const next = item ? fn(item) : null;
  return next ? { items: { ...s.items, [id]: next } } : s;
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
      nextItems[id] = { ...file, id, target: null, status: "idle", settings: { preset: "balanced" }, output: null, error: null };
      nextOrder.push(id);
    }

    const added = nextOrder.length - order.length;
    // Skip the set() entirely on a no-op drop so subscribers don't get a new `order` ref.
    if (added > 0) set({ items: nextItems, order: nextOrder });
    return added;
  },

  setTarget: (id, targetId) =>
    set((s) =>
      patch(s, id, (item) => {
        if (item.target === targetId || ACTIVE.has(item.status)) return null; // locked while converting
        // Guard against stale UI: the Rust matrix is the authority on what's legal.
        if (targetId !== null && !item.targets.some((t) => t.id === targetId)) return null;
        // A new target invalidates the previous result, so the row becomes startable again.
        return { ...item, target: targetId, status: "idle", output: null, error: null };
      }),
    ),

  setStatus: (id, status, extra) =>
    set((s) =>
      patch(s, id, (item) => {
        // Late progress events can arrive after completion; never regress a finished row.
        if (status === "running" && item.status !== "queued") return null;
        return { ...item, status, output: extra?.output ?? item.output, error: extra?.error ?? null };
      }),
    ),

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

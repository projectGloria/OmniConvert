import { create } from "zustand";
import type { JobId } from "../types/queue";
import { useQueue } from "./queueStore";

/**
 * Row selection, split from queueStore: clicking rows must not hand every queue
 * subscriber a new state object. Each row subscribes to its own boolean
 * (`s.selected[id] === true`), so a click re-renders exactly the rows that flipped.
 */
interface SelectionState {
  selected: Record<JobId, true>;
  /** Shift+click range origin (Explorer semantics: last plain/ctrl click). */
  anchor: JobId | null;
  click: (id: JobId, mode: "only" | "toggle" | "range") => void;
  selectAll: () => void;
  clear: () => void;
}

export const useSelection = create<SelectionState>()((set, get) => ({
  selected: {},
  anchor: null,

  click: (id, mode) => {
    const { selected, anchor } = get();
    if (mode === "toggle") {
      const next = { ...selected };
      if (next[id]) delete next[id];
      else next[id] = true;
      return set({ selected: next, anchor: id });
    }
    if (mode === "range" && anchor) {
      // Read order lazily: the range is resolved against the queue *as it is now*.
      const order = useQueue.getState().order;
      const [a, b] = [order.indexOf(anchor), order.indexOf(id)];
      if (a >= 0 && b >= 0) {
        const [lo, hi] = a < b ? [a, b] : [b, a];
        // Range replaces (Explorer behaviour), anchor stays put for the next shift-click.
        return set({ selected: Object.fromEntries(order.slice(lo, hi + 1).map((x) => [x, true] as const)) });
      }
    }
    // Plain click on the only selected row deselects it — a cheap "click again to undo".
    const only = Object.keys(selected).length === 1 && selected[id];
    set({ selected: only ? {} : { [id]: true }, anchor: id });
  },

  selectAll: () => set({ selected: Object.fromEntries(useQueue.getState().order.map((x) => [x, true] as const)) }),

  clear: () => set((s) => (Object.keys(s.selected).length ? { selected: {}, anchor: null } : s)),
}));

// Removed rows must leave the selection, or "Merge 3 to PDF" could count ghosts.
// A store-level subscription (not a component effect) so it holds no matter what's mounted.
useQueue.subscribe((q, prev) => {
  if (q.order === prev.order) return;
  const sel = useSelection.getState().selected;
  const stale = Object.keys(sel).filter((id) => !q.items[id]);
  if (stale.length === 0) return;
  const next = { ...sel };
  for (const id of stale) delete next[id];
  useSelection.setState({ selected: next });
});

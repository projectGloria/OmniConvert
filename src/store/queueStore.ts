import { create } from "zustand";
import { fieldsFor, pruneSettings } from "../lib/settingsSchema";
import type { FileInfo } from "../types/generated/FileInfo";
import type { JobSettings } from "../types/generated/JobSettings";
import { ACTIVE, FINAL, type ItemStatus, type JobId, type QueueItem } from "../types/queue";
import { useProgress } from "./progressStore";
import { fileItem, isMergeable, mergeItem } from "./queueItems";

/** Rows started together. The summary shows once all of them are FINAL. */
export interface Batch {
  ids: JobId[];
  startedAt: number;
  dismissed: boolean;
}

/** A settings edit. `undefined` = "back to the engine default" (key gets deleted). */
export type SettingsPatch = { [K in keyof JobSettings]?: JobSettings[K] | undefined };

export interface StatusExtra {
  output?: string;
  outputBytes?: number;
  error?: string;
  errorDetail?: string | null;
}

interface QueueState {
  /** Normalized map: a row update replaces one object, so only that row's selector fires. */
  items: Record<JobId, QueueItem>;
  /** Display order kept separately — reordering never touches item objects. */
  order: JobId[];
  batch: Batch | null;
  /** Returns how many files were actually added (duplicates by path are skipped). */
  addFiles: (files: FileInfo[]) => number;
  /** Appends one "Merge to PDF" row built from `ids` (in queue order). Returns its id. */
  addMerge: (ids: readonly JobId[]) => JobId | null;
  /** Pass null to clear. Ignores ids Rust didn't offer for this file. */
  setTarget: (id: JobId, targetId: string | null) => void;
  /** Sets `targetId` on this row and every same-format row that offers it.
   *  overwrite=false only fills rows without a target (unless none are left). Returns rows changed. */
  applyToAll: (id: JobId, targetId: string, overwrite: boolean) => number;
  /** Shallow-merges a settings patch; `undefined` values delete the key (back to default). */
  setSettings: (id: JobId, patch: SettingsPatch) => void;
  /** Lifecycle transitions driven by backend events (and by Convert). */
  setStatus: (id: JobId, status: ItemStatus, extra?: StatusExtra) => void;
  startBatch: (ids: readonly JobId[]) => void;
  dismissBatch: () => void;
  /** Active rows are skipped: they must be cancelled first. */
  remove: (ids: readonly JobId[]) => void;
  clear: () => void;
}

/** Immutable single-item patch; returns the same state object on no-op so nothing re-renders. */
function patch(s: QueueState, id: JobId, fn: (item: QueueItem) => QueueItem | null): QueueState | Partial<QueueState> {
  const item = s.items[id];
  const next = item ? fn(item) : null;
  return next ? { items: { ...s.items, [id]: next } } : s;
}

/** The one place a target changes: also resets the result and prunes stale settings. */
function retarget(item: QueueItem, targetId: string | null): QueueItem | null {
  if (item.target === targetId || ACTIVE.has(item.status)) return null; // locked while converting
  // Guard against stale UI: the Rust matrix is the authority on what's legal.
  const target = item.targets.find((t) => t.id === targetId);
  if (targetId !== null && !target) return null;
  const settings = pruneSettings(item.settings, fieldsFor(item, target));
  // A new target invalidates the previous result, so the row becomes startable again.
  return { ...item, target: targetId, settings, status: "idle", output: null, outputBytes: null, error: null, errorDetail: null };
}

export const useQueue = create<QueueState>()((set, get) => ({
  items: {},
  order: [],
  batch: null,

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
      nextItems[id] = fileItem(file, id);
      nextOrder.push(id);
    }
    const added = nextOrder.length - order.length;
    // Skip the set() entirely on a no-op drop so subscribers don't get a new `order` ref.
    if (added > 0) set({ items: nextItems, order: nextOrder });
    return added;
  },

  addMerge: (ids) => {
    const { items, order } = get();
    const wanted = new Set(ids);
    // Page order follows the queue's visual order, not click order — what you see is what you get.
    // Filter here, not in callers: a selection can mix a WAV in with the images, and the
    // header only *counted* mergeable rows — passing the raw selection made Rust reject it all.
    const sources = order.filter((id) => wanted.has(id)).map((id) => items[id]).filter(isMergeable);
    if (sources.length < 2) return null;
    const id = crypto.randomUUID();
    set({ items: { ...items, [id]: mergeItem(sources, id) }, order: [...order, id] });
    return id;
  },

  setTarget: (id, targetId) => set((s) => patch(s, id, (item) => retarget(item, targetId))),

  applyToAll: (id, targetId, overwrite) => {
    const { items } = get();
    const format = items[id]?.format;
    if (!format) return 0;
    const peers = Object.values(items).filter((it) => !it.merge && it.format === format && it.id !== id);
    // Nothing left to fill ⇒ the only useful meaning of "apply to all" is overwrite.
    const fill = !overwrite && peers.some((it) => it.target === null);
    const changed: Record<JobId, QueueItem> = {};
    for (const it of [items[id]!, ...peers]) {
      if (fill && it.target !== null && it.id !== id) continue;
      const next = retarget(it, targetId);
      if (next) changed[it.id] = next;
    }
    const n = Object.keys(changed).length;
    if (n > 0) set({ items: { ...items, ...changed } });
    return n;
  },

  setSettings: (id, change) =>
    set((s) =>
      patch(s, id, (item) => {
        if (ACTIVE.has(item.status)) return null; // the job already has its settings
        const merged: SettingsPatch = { ...item.settings, ...change };
        // exactOptionalPropertyTypes: "unset" must be an absent key, not `key: undefined` —
        // otherwise it would serialize away fine but break pruneSettings/hasOverrides checks.
        for (const k of Object.keys(merged) as (keyof JobSettings)[]) if (merged[k] === undefined) delete merged[k];
        return { ...item, settings: { ...merged, preset: merged.preset ?? "balanced" } as JobSettings };
      }),
    ),

  setStatus: (id, status, extra) =>
    set((s) =>
      patch(s, id, (item) => {
        // Late progress events can arrive after completion; never regress a finished row.
        if (status === "running" && item.status !== "queued") return null;
        return {
          ...item,
          status,
          output: extra?.output ?? (status === "queued" ? null : item.output),
          outputBytes: extra?.outputBytes ?? (status === "queued" ? null : item.outputBytes),
          error: extra?.error ?? null,
          errorDetail: extra?.errorDetail ?? null,
          finishedAt: FINAL.has(status) ? Date.now() : null,
        };
      }),
    ),

  startBatch: (ids) =>
    set((s) => {
      const b = s.batch;
      // Still running ⇒ grow the current batch (a Retry mid-batch belongs to it);
      // otherwise this is a fresh batch with its own clock.
      const live = b && !b.dismissed && b.ids.some((x) => ACTIVE.has(s.items[x]?.status ?? "idle"));
      return live
        ? { batch: { ...b, ids: [...new Set([...b.ids, ...ids])] } }
        : { batch: { ids: [...ids], startedAt: Date.now(), dismissed: false } };
    }),

  dismissBatch: () => set((s) => (s.batch ? { batch: { ...s.batch, dismissed: true } } : s)),

  remove: (ids) => {
    const { items } = get();
    const gone = new Set(ids.filter((id) => items[id] && !ACTIVE.has(items[id].status)));
    if (gone.size === 0) return;
    set((s) => ({
      items: Object.fromEntries(Object.entries(s.items).filter(([id]) => !gone.has(id))),
      order: s.order.filter((x) => !gone.has(x)),
    }));
    const { drop } = useProgress.getState();
    gone.forEach(drop); // keep the hot-path store from leaking dead entries
  },

  clear: () => {
    // "Clear all" must not orphan running jobs from the UI: keep active rows.
    get().remove(get().order);
  },
}));

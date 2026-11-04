import { AnimatePresence, motion } from "motion/react";
import { CheckCircle2, Download, FolderOpen, X } from "lucide-react";
import { platform } from "@platform";
import { useShallow } from "zustand/react/shallow";
import { formatBytes, formatDuration } from "../../lib/format";
import { useQueue } from "../../store/queueStore";
import { FINAL } from "../../types/queue";

interface Summary {
  done: number;
  failed: number;
  cancelled: number;
  inBytes: number;
  outBytes: number;
  ms: number;
  /** Whether any row produced an output (gates the reveal/download button). */
  firstOutput: string | null;
}

/**
 * Pure derivation from the queue: no "batchFinished" flag, no effect watching statuses.
 * Returns null while any batch row is still in flight, so the strip appears on the
 * exact render where the last row lands — and disappears if a Retry re-opens the batch.
 */
function summarize(s: ReturnType<typeof useQueue.getState>): Summary | null {
  const b = s.batch;
  if (!b || b.dismissed) return null;
  const rows = b.ids.flatMap((id) => (s.items[id] ? [s.items[id]] : [])); // removed rows drop out
  if (rows.length === 0 || !rows.every((r) => FINAL.has(r.status))) return null;

  const sum: Summary = { done: 0, failed: 0, cancelled: 0, inBytes: 0, outBytes: 0, ms: 0, firstOutput: null };
  for (const r of rows) {
    if (r.status === "done") {
      sum.done++;
      // Only successful rows count toward "in → out", or a failed 4 GB video would
      // make every batch look like a 99% size reduction.
      sum.inBytes += r.size;
      sum.outBytes += r.outputBytes ?? 0;
      sum.firstOutput ??= r.output;
    } else if (r.status === "error") sum.failed++;
    else sum.cancelled++;
    sum.ms = Math.max(sum.ms, (r.finishedAt ?? b.startedAt) - b.startedAt);
  }
  return sum;
}

/** Read at click time, not subscribed: the list only matters when the button is pressed. */
function batchOutputs(): string[] {
  const s = useQueue.getState();
  return (s.batch?.ids ?? []).flatMap((id) => (s.items[id]?.output ? [s.items[id].output] : []));
}

const RevealIcon = platform.kind === "web" ? Download : FolderOpen;

export function BatchSummary() {
  // useShallow compares the returned object's fields — progress ticks elsewhere in the
  // store produce an equal summary, so this strip doesn't re-render on them.
  const sum = useQueue(useShallow(summarize));
  const dismiss = useQueue((s) => s.dismissBatch);

  return (
    <AnimatePresence>
      {sum && (
        <motion.div
          initial={{ height: 0, opacity: 0 }}
          animate={{ height: "auto", opacity: 1 }}
          exit={{ height: 0, opacity: 0 }}
          transition={{ duration: 0.18 }}
          className="shrink-0 overflow-hidden border-t border-line bg-base"
        >
          <div role="status" className="flex h-10 items-center gap-3 px-4 text-xs">
            <CheckCircle2 size={14} className={sum.failed ? "text-danger" : "text-success"} />
            <span className="min-w-0 flex-1 truncate text-muted">
              <span className="text-ink">{sum.done} converted</span>
              {sum.failed > 0 && <span className="text-danger"> · {sum.failed} failed</span>}
              {sum.cancelled > 0 && <> · {sum.cancelled} cancelled</>}
              {sum.done > 0 && (
                <>
                  {" "}· {formatBytes(sum.inBytes)} → {formatBytes(sum.outBytes)}
                </>
              )}{" "}
              · {formatDuration(sum.ms)}
            </span>
            {sum.firstOutput && (
              <button
                type="button"
                onClick={() => void platform.revealAll(batchOutputs())}
                className="flex items-center gap-1.5 rounded-md px-2 py-1 text-ink transition-colors hover:bg-raised"
              >
                <RevealIcon size={13} className="text-muted" /> {platform.revealAllLabel}
              </button>
            )}
            <button type="button" aria-label="Dismiss summary" onClick={dismiss} className="grid size-6 place-items-center rounded text-muted hover:bg-raised hover:text-ink">
              <X size={13} />
            </button>
          </div>
        </motion.div>
      )}
    </AnimatePresence>
  );
}

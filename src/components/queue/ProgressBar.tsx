import { cn } from "../../lib/cn";
import { useProgress } from "../../store/progressStore";
import { useQueue } from "../../store/queueStore";
import type { ItemStatus, JobId } from "../../types/queue";

// One class per status. Stacking conflicting utilities (bg-accent + bg-muted) would let
// stylesheet order — not class order — decide the winner.
const BAR_COLOR: Record<ItemStatus, string> = {
  idle: "bg-accent",
  queued: "bg-accent",
  running: "bg-accent",
  done: "bg-success",
  error: "bg-danger",
  cancelled: "bg-muted/40",
};

/**
 * The only component (with RowStatus) subscribed to the hot progress store.
 *
 * Uses transform: scaleX instead of width — width changes trigger layout for the
 * whole row on every tick; a transform is compositor-only (no layout, no paint).
 */
export function ProgressBar({ id }: { id: JobId }) {
  const value = useProgress((s) => s.progress[id]?.value);
  const status = useQueue((s) => s.items[id]?.status);

  if (status === "idle" || status === undefined) return null;
  const indeterminate = status === "running" && value === null;

  return (
    <div className="absolute inset-x-0 bottom-0 h-0.5 overflow-hidden">
      {indeterminate ? (
        <div className="animate-indeterminate h-full w-2/5 bg-accent/80" />
      ) : (
        <div
          className={cn("h-full origin-left transition-[transform,background-color] duration-150 ease-out", BAR_COLOR[status])}
          style={{ transform: `scaleX(${status === "done" || status === "error" ? 1 : (value ?? 0)})` }}
        />
      )}
    </div>
  );
}

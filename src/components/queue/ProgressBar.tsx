import { useProgress } from "../../store/progressStore";
import type { JobId } from "../../types/queue";

/**
 * The only component subscribed to the hot progress store.
 *
 * Uses transform: scaleX instead of width — width changes trigger layout for the
 * whole row on every tick; a transform is compositor-only (no layout, no paint).
 */
export function ProgressBar({ id }: { id: JobId }) {
  const value = useProgress((s) => s.progress[id] ?? 0);

  return (
    <div className="absolute inset-x-0 bottom-0 h-0.5 bg-transparent">
      <div
        className="h-full origin-left bg-accent transition-transform duration-150 ease-out"
        style={{ transform: `scaleX(${value})` }}
      />
    </div>
  );
}

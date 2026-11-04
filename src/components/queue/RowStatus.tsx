import { formatBytes } from "../../lib/format";
import { useProgress } from "../../store/progressStore";
import type { QueueItem } from "../../types/queue";

/** Second line of a row: size + detection hint when idle, live stage/percent while converting. */
export function RowStatus({ item }: { item: QueueItem }) {
  // Hot store, but this component is tiny: a tick re-renders one line of text.
  const progress = useProgress((s) => s.progress[item.id]);

  switch (item.status) {
    case "queued":
      return <span className="text-muted">Queued</span>;
    case "running": {
      const pct = progress?.value == null ? "" : ` · ${Math.round(progress.value * 100)}%`;
      return <span className="text-accent">{(progress?.stage ?? "Starting") + pct}</span>;
    }
    case "done":
      return <span className="text-success">Done</span>;
    case "cancelled":
      return <span className="text-muted">Cancelled</span>;
    case "error":
      return (
        <span className="text-danger/90" title={item.error ?? undefined}>
          {item.error ?? "Failed"}
        </span>
      );
    default:
      return (
        <span className="text-muted">
          {formatBytes(item.size)}
          {/* Rust only sets detectedAs when magic bytes overrode the extension — no re-check here. */}
          {item.detectedAs && <span className="text-amber-400/80"> · detected as {item.detectedAs.toUpperCase()}</span>}
        </span>
      );
  }
}

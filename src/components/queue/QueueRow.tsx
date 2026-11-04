import { memo } from "react";
import { motion } from "motion/react";
import { ChevronDown, Settings2, X } from "lucide-react";
import { formatBytes } from "../../lib/format";
import { useQueue } from "../../store/queueStore";
import type { JobId } from "../../types/queue";
import { InputBadge } from "./InputBadge";
import { ProgressBar } from "./ProgressBar";

/**
 * memo + id-only prop: when QueueList re-renders because `order` changed (add/remove),
 * existing rows bail out — their only prop is a string that didn't change.
 */
export const QueueRow = memo(function QueueRow({ id }: { id: JobId }) {
  const item = useQueue((s) => s.items[id]);
  const remove = useQueue((s) => s.remove);

  // Can be undefined for one frame while AnimatePresence plays the exit animation.
  if (!item) return null;

  return (
    <motion.li
      layout="position"
      initial={{ opacity: 0, y: 6 }}
      animate={{ opacity: 1, y: 0 }}
      exit={{ opacity: 0, x: -16, transition: { duration: 0.15 } }}
      transition={{ type: "spring", stiffness: 500, damping: 38 }}
      className="group relative overflow-hidden rounded-lg border border-line bg-surface"
    >
      <div className="flex items-center gap-3 px-3.5 py-3">
        <InputBadge ext={item.ext} />

        <div className="min-w-0 flex-1">
          <p className="truncate text-sm text-ink" title={item.path}>
            {item.name}
          </p>
          <p className="text-xs text-muted">{formatBytes(item.size)}</p>
        </div>

        {/* Phase 2/4: becomes <FormatPill> backed by the Rust registry. */}
        <button
          type="button"
          disabled
          className="flex items-center gap-1.5 rounded-md border border-line bg-base px-2.5 py-1 text-xs text-muted disabled:cursor-not-allowed"
        >
          Choose format
          <ChevronDown size={13} />
        </button>

        <button
          type="button"
          disabled
          aria-label="Conversion settings"
          className="grid size-7 place-items-center rounded-md text-muted/50 disabled:cursor-not-allowed"
        >
          <Settings2 size={15} strokeWidth={1.75} />
        </button>

        <button
          type="button"
          aria-label={`Remove ${item.name}`}
          onClick={() => remove(id)}
          className="grid size-7 place-items-center rounded-md text-muted opacity-0 transition hover:bg-raised hover:text-ink focus-visible:opacity-100 group-hover:opacity-100"
        >
          <X size={15} />
        </button>
      </div>

      <ProgressBar id={id} />
    </motion.li>
  );
});

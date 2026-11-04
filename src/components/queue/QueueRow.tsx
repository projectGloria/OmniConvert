import { memo } from "react";
import { motion } from "motion/react";
import { Settings2, X } from "lucide-react";
import { formatBytes } from "../../lib/format";
import { useQueue } from "../../store/queueStore";
import type { JobId } from "../../types/queue";
import { FormatPill } from "../picker/FormatPill";
import { InputBadge } from "./InputBadge";
import { ProgressBar } from "./ProgressBar";

/**
 * memo + id-only prop: when QueueList re-renders because `order` changed (add/remove),
 * existing rows bail out — their only prop is a string that didn't change.
 */
export const QueueRow = memo(function QueueRow({ id }: { id: JobId }) {
  const item = useQueue((s) => s.items[id]);
  const remove = useQueue((s) => s.remove);
  const setTarget = useQueue((s) => s.setTarget);

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
        <InputBadge ext={item.ext} category={item.category} />

        <div className="min-w-0 flex-1">
          {/* w-fit: the title tooltip should trigger on the filename, not the row's empty space. */}
          <p className="w-fit max-w-full truncate text-sm text-ink" title={item.path}>
            {item.name}
          </p>
          <p className="text-xs text-muted">
            {formatBytes(item.size)}
            {/* Rust only sets detectedAs when magic bytes overrode the extension — no re-check here. */}
            {item.detectedAs && (
              <span className="text-amber-400/80"> · detected as {item.detectedAs.toUpperCase()}</span>
            )}
          </p>
        </div>

        <FormatPill targets={item.targets} selectedId={item.target} onSelect={(t) => setTarget(id, t)} />

        {/* Phase 4: opens the per-item settings drawer. */}
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

import { memo } from "react";
import { motion } from "motion/react";
import { useQueue } from "../../store/queueStore";
import { ACTIVE, type JobId } from "../../types/queue";
import { FormatPill } from "../picker/FormatPill";
import { InputBadge } from "./InputBadge";
import { ProgressBar } from "./ProgressBar";
import { RowActions } from "./RowActions";
import { RowStatus } from "./RowStatus";

/**
 * memo + id-only prop: when QueueList re-renders because `order` changed (add/remove),
 * existing rows bail out — their only prop is a string that didn't change.
 * Progress ticks don't reach this component at all (see progressStore).
 */
export const QueueRow = memo(function QueueRow({ id }: { id: JobId }) {
  const item = useQueue((s) => s.items[id]);
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
          <p className="truncate text-xs">
            <RowStatus item={item} />
          </p>
        </div>

        <FormatPill
          targets={item.targets}
          selectedId={item.target}
          locked={ACTIVE.has(item.status)}
          onSelect={(t) => setTarget(id, t)}
        />
        <RowActions item={item} />
      </div>

      <ProgressBar id={id} />
    </motion.li>
  );
});

import { memo, useState, type MouseEvent } from "react";
import { AnimatePresence, motion } from "motion/react";
import { platform } from "@platform";
import { cn } from "../../lib/cn";
import { useApp } from "../../store/appStore";
import { useQueue } from "../../store/queueStore";
import { useSelection } from "../../store/selectionStore";
import { ACTIVE, type JobId } from "../../types/queue";
import { FormatPill } from "../picker/FormatPill";
import { InputBadge } from "./InputBadge";
import { ProgressBar } from "./ProgressBar";
import { RowActions } from "./RowActions";
import { RowStatus } from "./RowStatus";

/**
 * Clicks on controls must not also (de)select the row. Two traps: React synthetic events
 * bubble through *portals* by component tree, so a click inside the floating picker or
 * settings drawer (rendered in <body>) still reaches this onClick — hence the DOM
 * containment check; and clicking a <label> targets the label, not its hidden radio.
 */
const fromControl = (e: MouseEvent) => {
  const el = e.target as HTMLElement;
  return !e.currentTarget.contains(el) || el.closest("button, input, label, a") !== null;
};

/**
 * memo + id-only prop: when QueueList re-renders because `order` changed (add/remove),
 * existing rows bail out — their only prop is a string that didn't change.
 * Progress ticks don't reach this component at all (see progressStore).
 */
export const QueueRow = memo(function QueueRow({ id }: { id: JobId }) {
  const item = useQueue((s) => s.items[id]);
  const setTarget = useQueue((s) => s.setTarget);
  const applyToAll = useQueue((s) => s.applyToAll);
  // Boolean selector: toggling row A never re-renders row B.
  const selected = useSelection((s) => s.selected[id] === true);
  const click = useSelection((s) => s.click);
  const [showDetail, setShowDetail] = useState(false);

  // Can be undefined for one frame while AnimatePresence plays the exit animation.
  if (!item) return null;
  const detail = item.status === "error" ? item.errorDetail : null;

  const onClick = (e: MouseEvent) => {
    if (fromControl(e)) return;
    click(id, e.shiftKey ? "range" : e.ctrlKey || e.metaKey ? "toggle" : "only");
  };

  return (
    <motion.li
      layout="position"
      initial={{ opacity: 0, y: 6 }}
      animate={{ opacity: 1, y: 0 }}
      exit={{ opacity: 0, x: -16, transition: { duration: 0.15 } }}
      transition={{ type: "spring", stiffness: 500, damping: 38 }}
      // A focusable option in a multi-select listbox: Space toggles, Del removes (useHotkeys).
      role="option"
      aria-selected={selected}
      tabIndex={0}
      data-row-id={id}
      onClick={onClick}
      onKeyDown={(e) => {
        if (e.target !== e.currentTarget) return; // keys typed into the row's controls
        if (e.key === " ") {
          e.preventDefault(); // no page scroll
          click(id, "toggle");
        } else if (e.key === "ArrowDown" || e.key === "ArrowUp") {
          // Row-to-row focus without Tab-ing through every pill and gear in between.
          e.preventDefault();
          const sib = e.key === "ArrowDown" ? e.currentTarget.nextElementSibling : e.currentTarget.previousElementSibling;
          (sib as HTMLElement | null)?.focus();
          if (e.shiftKey && sib instanceof HTMLElement && sib.dataset.rowId) click(sib.dataset.rowId, "range");
        }
      }}
      className={cn(
        "group relative overflow-hidden rounded-lg border bg-surface outline-none select-none",
        "focus-visible:ring-1 focus-visible:ring-accent/70",
        selected ? "border-accent/50 bg-accent/[0.06]" : "border-line",
      )}
    >
      <div className="flex items-center gap-3 px-3.5 py-3">
        <InputBadge ext={item.ext} category={item.category} />

        <div className="min-w-0 flex-1">
          {/* w-fit: the title tooltip should trigger on the filename, not the row's empty space. */}
          <p className="w-fit max-w-full truncate text-sm text-ink" title={item.merge ? item.merge.join("\n") : item.path}>
            {item.name}
          </p>
          <p className="flex min-w-0 items-center gap-2 truncate text-xs">
            <RowStatus item={item} />
            {detail && (
              <button
                type="button"
                aria-expanded={showDetail}
                onClick={() => setShowDetail((v) => !v)}
                className="shrink-0 text-muted underline decoration-dotted underline-offset-2 hover:text-ink"
              >
                {showDetail ? "hide details" : "details"}
              </button>
            )}
          </p>
        </div>

        <FormatPill
          targets={item.targets}
          // Web: a recognised file with no targets is a sidecar route (video, audio,
          // documents), not an unknown format — say where it *does* convert.
          emptyLabel={item.format && platform.kind === "web" ? "Desktop app only" : undefined}
          selectedId={item.target}
          locked={ACTIVE.has(item.status) || item.merge !== null}
          onSelect={(t) => setTarget(id, t)}
          applyAll={
            item.format && !item.merge
              ? {
                  rowId: id,
                  format: item.format,
                  onApply: (t, overwrite) => {
                    const n = applyToAll(id, t, overwrite);
                    if (n > 1) useApp.getState().notify(`Applied to ${n} .${item.format} files`);
                  },
                }
              : undefined
          }
        />
        <RowActions item={item} />
      </div>

      <AnimatePresence initial={false}>
        {detail && showDetail && (
          <motion.pre
            initial={{ height: 0, opacity: 0 }}
            animate={{ height: "auto", opacity: 1 }}
            exit={{ height: 0, opacity: 0 }}
            className="mx-3.5 mb-3 max-h-40 overflow-auto rounded-md border border-line bg-base p-2.5 font-mono text-[11px] leading-relaxed whitespace-pre-wrap text-muted select-text"
          >
            {detail}
          </motion.pre>
        )}
      </AnimatePresence>

      <ProgressBar id={id} />
    </motion.li>
  );
});

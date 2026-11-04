import { CopyCheck } from "lucide-react";
import { useShallow } from "zustand/react/shallow";
import { useQueue } from "../../store/queueStore";
import type { Format } from "../../types/generated/Format";
import type { Target } from "../../types/generated/Target";
import { ACTIVE, type JobId } from "../../types/queue";

export interface ApplyAllProps {
  rowId: JobId;
  format: Format;
  /** The highlighted option — what "apply" would apply. */
  target: Target | undefined;
  onApply: (targetId: string, overwrite: boolean) => void;
}

/**
 * "Apply to all .png files". Click fills rows that have no target yet; Shift+click
 * (or Alt+Shift+Enter in the search box) overwrites rows that already chose one.
 *
 * Only mounted while a popover is open, so its O(n) count selector runs for one row
 * at a time — never per row per store update across a 5,000-file queue.
 */
export function ApplyAllFooter({ rowId, format, target, onApply }: ApplyAllProps) {
  const targetId = target?.id;
  const [empty, total] = useQueue(
    // useShallow: the selector builds a fresh tuple each run; compare its contents.
    useShallow((s) => {
      let e = 0;
      let t = 0;
      for (const id of s.order) {
        const it = s.items[id];
        // Same-format rows can still differ (an animated GIF offers video targets) — count
        // only rows that actually offer this target and aren't mid-conversion.
        if (!it || id === rowId || it.merge || it.format !== format || ACTIVE.has(it.status)) continue;
        if (!it.targets.some((x) => x.id === targetId)) continue;
        t++;
        if (it.target === null) e++;
      }
      return [e, t] as const;
    }),
  );

  if (!target || total === 0) return null;

  return (
    <button
      type="button"
      onMouseDown={(e) => e.preventDefault()} // keep focus (and the highlight) in the search box
      onClick={(e) => onApply(target.id, e.shiftKey)}
      title="Shift+click to also overwrite rows that already have a format · Alt+Enter from the search box"
      className="flex items-center gap-2 border-t border-line px-3 py-2 text-left text-xs text-muted transition-colors hover:bg-raised hover:text-ink"
    >
      <CopyCheck size={13} className="shrink-0 text-accent" />
      <span className="min-w-0 flex-1 truncate">
        Apply <span className="text-ink">{target.label}</span> to {empty > 0 ? empty : total} other .{format}
        {/* Mirrors the store: fill-only while blanks remain, overwrite once none do. */}
        {empty > 0 && empty < total ? ` (Shift: all ${total})` : ""}
      </span>
      <kbd className="shrink-0 font-sans text-[10px] text-muted/70">Alt+↵</kbd>
    </button>
  );
}

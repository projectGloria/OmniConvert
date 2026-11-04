import { FilePlus2, Files } from "lucide-react";
import { useShallow } from "zustand/react/shallow";
import { browse } from "../../lib/ingest";
import { isMergeable } from "../../store/queueItems";
import { useQueue } from "../../store/queueStore";
import { useSelection } from "../../store/selectionStore";
import { ACTIVE } from "../../types/queue";

const LINK = "flex items-center gap-1.5 text-xs text-muted transition-colors hover:text-ink";

export function QueueHeader({ count }: { count: number }) {
  // useShallow: Object.keys() is a new array each run; compare element-wise so this
  // header only re-renders when the *set* of selected ids actually changes.
  const selectedIds = useSelection(useShallow((s) => Object.keys(s.selected)));
  // Derived, not stored: how many of the selected rows the merge engine can take.
  const mergeable = useQueue((s) =>
    selectedIds.reduce((n, id) => {
      const it = s.items[id];
      return isMergeable(it) && !ACTIVE.has(it.status) ? n + 1 : n;
    }, 0),
  );
  const clear = useQueue((s) => s.clear);

  const merge = () => {
    const id = useQueue.getState().addMerge(selectedIds);
    if (id) {
      useSelection.getState().click(id, "only"); // focus attention on the new row
      // The new row isn't in the DOM until React commits the store update — wait a frame.
      requestAnimationFrame(() => document.querySelector(`[data-row-id="${id}"]`)?.scrollIntoView({ block: "nearest" }));
    }
  };

  return (
    <div className="flex h-11 items-center justify-between gap-3 px-5 pt-2">
      <span className="text-xs font-medium tracking-wide text-muted uppercase">
        Queue · {count} {count === 1 ? "file" : "files"}
        {selectedIds.length > 0 && <span className="text-accent normal-case"> · {selectedIds.length} selected</span>}
      </span>

      <div className="flex items-center gap-4">
        {mergeable >= 2 && (
          <button
            type="button"
            onClick={merge}
            title="Combine the selected images into one PDF, one page each, in queue order"
            className="flex items-center gap-1.5 rounded-md border border-accent/40 bg-accent/10 px-2 py-1 text-xs text-ink transition-colors hover:bg-accent/20"
          >
            <Files size={13} className="text-accent" />
            Merge {mergeable} into PDF
          </button>
        )}
        <button type="button" onClick={() => void browse()} className={LINK} title="Add files (Ctrl+O)">
          <FilePlus2 size={13} /> Add
        </button>
        <button type="button" onClick={clear} className={LINK} title="Remove every row that isn't converting">
          Clear all
        </button>
      </div>
    </div>
  );
}

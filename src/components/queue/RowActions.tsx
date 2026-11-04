import { revealItemInDir } from "@tauri-apps/plugin-opener";
import { FolderSearch, Settings2, Square, X } from "lucide-react";
import { cancelJob } from "../../lib/ipc";
import { useQueue } from "../../store/queueStore";
import { ACTIVE, type QueueItem } from "../../types/queue";

const ICON_BTN = "grid size-7 place-items-center rounded-md text-muted transition hover:bg-raised hover:text-ink";

export function RowActions({ item }: { item: QueueItem }) {
  const remove = useQueue((s) => s.remove);
  const active = ACTIVE.has(item.status);

  return (
    <>
      {item.status === "done" && item.output && (
        <button
          type="button"
          title="Show in Explorer"
          aria-label="Show in Explorer"
          onClick={() => void revealItemInDir(item.output!)}
          className={`${ICON_BTN} text-success hover:text-success`}
        >
          <FolderSearch size={15} strokeWidth={1.75} />
        </button>
      )}

      {/* Phase 4: opens the per-item settings drawer. */}
      <button type="button" disabled aria-label="Conversion settings" className="grid size-7 place-items-center rounded-md text-muted/40 disabled:cursor-not-allowed">
        <Settings2 size={15} strokeWidth={1.75} />
      </button>

      {active ? (
        // Cancel is optimistic-free: the row flips to "Cancelled" when Rust confirms
        // via conversion://error{cancelled}, so UI never claims a kill that didn't happen.
        <button type="button" title="Cancel" aria-label={`Cancel ${item.name}`} onClick={() => void cancelJob(item.id)} className={ICON_BTN}>
          <Square size={12} strokeWidth={2} className="fill-current" />
        </button>
      ) : (
        <button
          type="button"
          aria-label={`Remove ${item.name}`}
          onClick={() => remove(item.id)}
          className={`${ICON_BTN} opacity-0 focus-visible:opacity-100 group-hover:opacity-100`}
        >
          <X size={15} />
        </button>
      )}
    </>
  );
}

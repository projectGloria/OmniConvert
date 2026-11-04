import { revealItemInDir } from "@tauri-apps/plugin-opener";
import { FolderSearch, RotateCcw, Square, X } from "lucide-react";
import { convert } from "../../lib/convert";
import { cancelJob } from "../../lib/ipc";
import { useQueue } from "../../store/queueStore";
import { ACTIVE, type QueueItem } from "../../types/queue";
import { SettingsButton } from "../settings/SettingsButton";

const ICON_BTN = "grid size-7 place-items-center rounded-md text-muted transition hover:bg-raised hover:text-ink";

export function RowActions({ item }: { item: QueueItem }) {
  const remove = useQueue((s) => s.remove);
  const active = ACTIVE.has(item.status);
  const retryable = (item.status === "error" || item.status === "cancelled") && item.target !== null;

  return (
    <>
      {item.status === "done" && item.output && (
        <button
          type="button"
          title="Show in Explorer"
          aria-label={`Show ${item.name} output in Explorer`}
          onClick={() => void revealItemInDir(item.output!)}
          className={`${ICON_BTN} text-success hover:text-success`}
        >
          <FolderSearch size={15} strokeWidth={1.75} />
        </button>
      )}

      {retryable && (
        // Retry = convert just this row. It joins the live batch if one is running.
        <button type="button" title="Retry" aria-label={`Retry ${item.name}`} onClick={() => void convert([item.id])} className={ICON_BTN}>
          <RotateCcw size={14} strokeWidth={1.75} />
        </button>
      )}

      <SettingsButton item={item} />

      {active ? (
        // Cancel is optimistic-free: the row flips to "Cancelled" when Rust confirms
        // via conversion://error{cancelled}, so UI never claims a kill that didn't happen.
        <button type="button" title="Cancel" aria-label={`Cancel ${item.name}`} onClick={() => void cancelJob(item.id)} className={ICON_BTN}>
          <Square size={12} strokeWidth={2} className="fill-current" />
        </button>
      ) : (
        <button
          type="button"
          title="Remove (Del)"
          aria-label={`Remove ${item.name}`}
          onClick={() => remove([item.id])}
          className={`${ICON_BTN} opacity-0 group-focus-within:opacity-100 group-hover:opacity-100 focus-visible:opacity-100`}
        >
          <X size={15} />
        </button>
      )}
    </>
  );
}

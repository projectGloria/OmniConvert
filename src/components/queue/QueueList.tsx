import { AnimatePresence } from "motion/react";
import { useQueue } from "../../store/queueStore";
import { QueueRow } from "./QueueRow";

export function QueueList() {
  // Subscribes to `order` only. Row contents are read inside each memoized QueueRow,
  // so editing one item re-renders one row — not the list.
  const order = useQueue((s) => s.order);
  const clear = useQueue((s) => s.clear);

  return (
    <div className="flex h-full flex-col">
      <div className="flex items-center justify-between px-5 pt-4 pb-2">
        <span className="text-xs font-medium tracking-wide text-muted uppercase">
          Queue · {order.length} {order.length === 1 ? "file" : "files"}
        </span>
        <button type="button" onClick={clear} className="text-xs text-muted transition-colors hover:text-ink">
          Clear all
        </button>
      </div>
      <ul className="min-h-0 flex-1 space-y-2 overflow-y-auto px-4 pb-4">
        {/* initial={false}: rows already present on mount don't replay their enter animation. */}
        <AnimatePresence initial={false}>
          {order.map((id) => (
            <QueueRow key={id} id={id} />
          ))}
        </AnimatePresence>
      </ul>
    </div>
  );
}

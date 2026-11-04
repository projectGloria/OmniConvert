import { AnimatePresence } from "motion/react";
import { useQueue } from "../../store/queueStore";
import { QueueHeader } from "./QueueHeader";
import { QueueRow } from "./QueueRow";

export function QueueList() {
  // Subscribes to `order` only. Row contents are read inside each memoized QueueRow,
  // so editing one item re-renders one row — not the list.
  const order = useQueue((s) => s.order);

  return (
    <div className="flex h-full flex-col">
      <QueueHeader count={order.length} />
      <ul role="listbox" aria-multiselectable aria-label="Conversion queue" className="min-h-0 flex-1 space-y-2 overflow-y-auto px-4 pb-4">
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

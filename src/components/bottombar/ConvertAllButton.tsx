import { ArrowRight, Square } from "lucide-react";
import { convertAll } from "../../lib/convert";
import { cancelAll } from "../../lib/ipc";
import { useQueue } from "../../store/queueStore";
import { ACTIVE, STARTABLE } from "../../types/queue";

export function ConvertAllButton() {
  // Derived, not stored: both counts are pure functions of the queue. A separate
  // useState + useEffect sync would be one render late and a source of drift.
  // Primitive selector results ⇒ Zustand's Object.is check skips no-op renders.
  const ready = useQueue((s) =>
    s.order.reduce((n, id) => {
      const it = s.items[id];
      return it?.target && STARTABLE.has(it.status) ? n + 1 : n;
    }, 0),
  );
  const active = useQueue((s) => s.order.reduce((n, id) => (ACTIVE.has(s.items[id]?.status ?? "idle") ? n + 1 : n), 0));
  // Tooltip only makes sense if something is actually missing a target (not when all are done).
  const untargeted = useQueue((s) => s.order.some((id) => !s.items[id]?.target));

  if (active > 0 && ready === 0) {
    return (
      <button
        type="button"
        onClick={() => void cancelAll()}
        className="flex shrink-0 items-center gap-2 rounded-lg border border-line bg-raised px-4 py-2 text-sm text-ink transition hover:border-danger/60 hover:text-danger"
      >
        <Square size={11} className="fill-current" />
        Cancel all ({active})
      </button>
    );
  }

  const disabled = ready === 0;
  return (
    <button
      type="button"
      disabled={disabled}
      onClick={() => void convertAll()}
      title={disabled && untargeted ? "Pick a target format first" : undefined}
      className="flex shrink-0 items-center gap-2 rounded-lg bg-accent px-4 py-2 text-sm font-medium text-white shadow-[0_0_0_1px_rgba(255,255,255,0.06)_inset] transition hover:brightness-110 active:brightness-95 disabled:cursor-not-allowed disabled:bg-raised disabled:text-muted disabled:shadow-none"
    >
      Convert {ready > 0 ? ready : "All"}
      <ArrowRight size={15} />
    </button>
  );
}

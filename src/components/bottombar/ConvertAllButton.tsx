import { ArrowRight } from "lucide-react";
import { useQueue } from "../../store/queueStore";

export function ConvertAllButton() {
  // Derived, not stored: "ready count" is a pure function of the queue. A separate
  // useState + useEffect sync would be one render late and a source of drift.
  // Selector returns a primitive, so Zustand's Object.is check keeps re-renders minimal.
  const ready = useQueue((s) => s.order.reduce((n, id) => (s.items[id]?.target ? n + 1 : n), 0));
  const total = useQueue((s) => s.order.length);

  const disabled = ready === 0;

  return (
    <button
      type="button"
      disabled={disabled}
      title={disabled && total > 0 ? "Pick a target format first" : undefined}
      className="flex shrink-0 items-center gap-2 rounded-lg bg-accent px-4 py-2 text-sm font-medium text-white shadow-[0_0_0_1px_rgba(255,255,255,0.06)_inset] transition hover:brightness-110 active:brightness-95 disabled:cursor-not-allowed disabled:bg-raised disabled:text-muted disabled:shadow-none"
    >
      Convert {ready > 0 ? ready : "All"}
      <ArrowRight size={15} />
    </button>
  );
}

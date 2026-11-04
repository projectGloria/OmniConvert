import { useEffect } from "react";
import { AnimatePresence, motion } from "motion/react";
import { cn } from "../lib/cn";
import { useApp } from "../store/appStore";

const TTL_MS = 3500;

/** One toast at a time: newer replaces older (no stack for a converter's tiny notices). */
export function Toast() {
  const toast = useApp((s) => s.toast);
  const dismiss = useApp((s) => s.dismissToast);

  // A timer is a real side effect: re-armed per toast id, cleared if replaced first.
  useEffect(() => {
    if (!toast) return;
    const t = window.setTimeout(() => dismiss(toast.id), toast.tone === "error" ? TTL_MS * 2 : TTL_MS);
    return () => window.clearTimeout(t);
  }, [toast, dismiss]);

  return (
    <div aria-live="polite" className="pointer-events-none fixed inset-x-0 bottom-20 z-50 flex justify-center">
      <AnimatePresence>
        {toast && (
          <motion.div
            key={toast.id}
            initial={{ opacity: 0, y: 8 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0, y: 8 }}
            onClick={() => dismiss(toast.id)}
            className={cn(
              "pointer-events-auto max-w-md rounded-lg border bg-raised px-3.5 py-2 text-xs shadow-xl shadow-black/50",
              toast.tone === "error" ? "border-danger/40 text-danger" : "border-line text-ink",
            )}
          >
            {toast.text}
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
}

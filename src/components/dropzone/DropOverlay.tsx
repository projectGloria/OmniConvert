import { AnimatePresence, motion } from "motion/react";
import { Download } from "lucide-react";

/**
 * Full-window drop affordance. Purely visual — the actual drop is handled by the
 * native Tauri listener in useFileDrop, so this layer uses pointer-events-none and
 * never intercepts anything.
 */
export function DropOverlay({ visible }: { visible: boolean }) {
  return (
    <AnimatePresence>
      {visible && (
        <motion.div
          key="drop-overlay"
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          exit={{ opacity: 0 }}
          transition={{ duration: 0.12 }}
          className="pointer-events-none fixed inset-x-0 top-9 bottom-0 z-50 p-3"
        >
          <div className="grid h-full place-items-center rounded-xl border-2 border-dashed border-accent/70 bg-base/85 backdrop-blur-sm">
            <motion.div
              initial={{ scale: 0.94, y: 6 }}
              animate={{ scale: 1, y: 0 }}
              transition={{ type: "spring", stiffness: 420, damping: 30 }}
              className="flex flex-col items-center gap-3"
            >
              <div className="grid size-14 place-items-center rounded-2xl bg-accent/15 text-accent">
                <Download size={26} strokeWidth={1.75} />
              </div>
              <p className="text-sm font-medium text-ink">Release to add files</p>
              <p className="text-xs text-muted">Folders are scanned recursively</p>
            </motion.div>
          </div>
        </motion.div>
      )}
    </AnimatePresence>
  );
}

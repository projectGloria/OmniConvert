import { useEffect } from "react";
import { convert } from "../lib/convert";
import { browse } from "../lib/ingest";
import { useQueue } from "../store/queueStore";
import { useSelection } from "../store/selectionStore";

/** Typing contexts where single keys (Del, Ctrl+A, Esc) belong to the field, not the app. */
const isEditable = (t: EventTarget | null) =>
  t instanceof HTMLElement && (t.isContentEditable || t.closest("input, textarea, select") !== null);

/**
 * App-wide shortcuts on one window listener. Everything is read via getState() at
 * keypress time, so the effect never re-subscribes and can't capture stale state.
 *
 *   Ctrl+O      add files          Ctrl+Enter  convert all
 *   Ctrl+A      select all rows    Del         remove selected (idle/finished only)
 *   Esc         clear selection    (popovers handle their own Esc via floating-ui)
 */
export function useHotkeys(): void {
  useEffect(() => {
    let browsing = false; // a second Ctrl+O while the dialog is open would stack dialogs

    const onKey = (e: KeyboardEvent) => {
      // A popover's search box already handled Enter (and stopped it), or the event is a repeat.
      if (e.defaultPrevented || e.repeat) return;
      const ctrl = e.ctrlKey || e.metaKey;
      const editing = isEditable(e.target);

      if (ctrl && e.key.toLowerCase() === "o") {
        e.preventDefault(); // WebView2 would otherwise show its own "open file" behaviour
        if (browsing) return;
        browsing = true;
        void browse().finally(() => (browsing = false));
      } else if (ctrl && e.key === "Enter") {
        e.preventDefault();
        void convert();
      } else if (editing) {
        return;
      } else if (ctrl && e.key.toLowerCase() === "a") {
        e.preventDefault(); // instead of selecting all page text
        useSelection.getState().selectAll();
      } else if (e.key === "Delete") {
        const ids = Object.keys(useSelection.getState().selected);
        if (ids.length) useQueue.getState().remove(ids);
      } else if (e.key === "Escape") {
        useSelection.getState().clear();
      }
    };

    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);
}

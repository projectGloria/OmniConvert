import { useEffect, useState } from "react";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { ingest } from "../lib/ingest";

/**
 * Subscribes to Tauri's *native* drag-drop events (not HTML5 dragenter/drop).
 * With `dragDropEnabled: true` WebView2 hands us absolute OS paths — the browser
 * File API would only give us blobs, useless for ffmpeg/pandoc.
 *
 * Returns whether files are currently hovering over the window.
 */
export function useFileDrop(): boolean {
  const [hovering, setHovering] = useState(false);
  useEffect(() => {
    // onDragDropEvent resolves its unlisten fn *asynchronously*. Under StrictMode the
    // effect is torn down before that promise settles, so a naive `return unlisten`
    // would leak the first listener and every drop would be handled twice.
    let disposed = false;
    let unlisten: (() => void) | undefined;

    getCurrentWebview()
      .onDragDropEvent(async ({ payload }) => {
        switch (payload.type) {
          case "enter":
            // Dragging text/links from another app yields zero paths — don't show the overlay.
            if (payload.paths.length > 0) setHovering(true);
            break;
          case "leave":
            setHovering(false);
            break;
          case "drop":
            setHovering(false);
            await ingest(payload.paths); // empty-paths + error toasts handled there
            break;
          // "over" fires on every mouse move; nothing to do.
        }
      })
      .then((fn) => {
        if (disposed) fn();
        else unlisten = fn;
      });

    return () => {
      disposed = true;
      unlisten?.();
    };
  }, []);

  return hovering;
}

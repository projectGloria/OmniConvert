import { useEffect, useState } from "react";
import { platform } from "@platform";
import { ingest } from "../lib/ingest";

/**
 * Window-wide file drop. Desktop: Tauri's *native* drag-drop (real OS paths, which
 * ffmpeg/pandoc need). Web: HTML5 drop, files registered under synthetic path keys.
 *
 * Returns whether files are currently hovering over the window.
 */
export function useFileDrop(): boolean {
  const [hovering, setHovering] = useState(false);
  useEffect(() => {
    // onFileDrop resolves its unlisten fn *asynchronously*. Under StrictMode the
    // effect is torn down before that promise settles, so a naive `return unlisten`
    // would leak the first listener and every drop would be handled twice.
    let disposed = false;
    let unlisten: (() => void) | undefined;

    void platform
      .onFileDrop({
        enter: () => setHovering(true),
        leave: () => setHovering(false),
        drop: (paths) => {
          setHovering(false);
          void ingest(paths); // empty-paths + error toasts handled there
        },
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

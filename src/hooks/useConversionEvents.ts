import { useEffect } from "react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { CompleteEvent } from "../types/generated/CompleteEvent";
import type { ErrorEvent } from "../types/generated/ErrorEvent";
import type { ProgressEvent } from "../types/generated/ProgressEvent";
import { useProgress } from "../store/progressStore";
import { useQueue } from "../store/queueStore";

/**
 * Bridges backend job events into the stores. Mounted once at the app root.
 *
 * Handlers read actions via getState() instead of closing over hook values: the
 * listeners live for the app's lifetime, and a captured selector value would go stale
 * (the classic stale-closure bug). Store actions are stable, so this is always fresh.
 */
export function useConversionEvents(): void {
  useEffect(() => {
    let disposed = false;
    const unlisteners: UnlistenFn[] = [];
    // listen() resolves asynchronously; under StrictMode cleanup can run first, so
    // late-arriving unlisten fns are invoked immediately instead of leaking.
    const keep = (p: Promise<UnlistenFn>) =>
      void p.then((fn) => (disposed ? fn() : unlisteners.push(fn)));

    keep(
      listen<ProgressEvent>("conversion://progress", ({ payload }) => {
        useProgress.getState().setProgress(payload.jobId, { value: payload.progress, stage: payload.stage });
        useQueue.getState().setStatus(payload.jobId, "running");
      }),
    );
    keep(
      listen<CompleteEvent>("conversion://complete", ({ payload }) => {
        useProgress.getState().setProgress(payload.jobId, { value: 1, stage: "Done" });
        useQueue.getState().setStatus(payload.jobId, "done", { output: payload.output, outputBytes: payload.outputBytes });
      }),
    );
    keep(
      listen<ErrorEvent>("conversion://error", ({ payload }) => {
        useProgress.getState().drop(payload.jobId);
        useQueue
          .getState()
          .setStatus(payload.jobId, payload.cancelled ? "cancelled" : "error", {
            error: payload.message,
            errorDetail: payload.detail,
          });
      }),
    );

    return () => {
      disposed = true;
      unlisteners.forEach((fn) => fn());
    };
  }, []);
}

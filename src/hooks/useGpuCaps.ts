import { useEffect } from "react";
import { platform } from "@platform";
import { gpuCaps } from "../lib/ipc";
import { useApp } from "../store/appStore";
import type { GpuCaps } from "../types/generated/GpuCaps";

/**
 * Mirrors the backend's GPU probe into appStore. Two sources on purpose: the event
 * may have fired *before* this listener attached (probe finished while the webview
 * was still loading), and the pull may run before the probe finished. Whichever says
 * `ready: true` wins; a non-ready pull can never clobber a ready event.
 */
export function useGpuCaps(): void {
  useEffect(() => {
    let disposed = false;
    let unlisten: (() => void) | undefined;
    const accept = (caps: GpuCaps) => {
      if (!disposed && (caps.ready || !useApp.getState().gpu.ready)) useApp.getState().setGpu(caps);
    };

    void platform.listen<GpuCaps>("system://gpu-ready", accept).then((fn) => {
      if (disposed) fn(); // StrictMode: cleanup already ran
      else unlisten = fn;
    });
    gpuCaps().then(accept, () => {}); // a failed pull just means "wait for the event"

    return () => {
      disposed = true;
      unlisten?.();
    };
  }, []);
}

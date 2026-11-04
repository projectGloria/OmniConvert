import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { open } from "@tauri-apps/plugin-dialog";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import type { FileInfo } from "../../types/generated/FileInfo";
import type { GpuCaps } from "../../types/generated/GpuCaps";
import type { Platform } from "../types";

// Desktop host: thin typed wrappers over Tauri commands/events. A renamed Rust command
// breaks here — in one place — instead of at a call site at runtime.
export const platform: Platform = {
  kind: "desktop",
  hasWindowControls: true,
  revealLabel: "Show in Explorer",
  revealAllLabel: "Open output folder",

  probeFiles: (paths) => invoke<FileInfo[]>("probe_files", { paths }),
  // Fire-and-forget; results arrive as conversion://progress|complete|error events.
  enqueueJobs: (jobs) => invoke<void>("enqueue_jobs", { jobs }),
  enqueueMerge: (req) => invoke<void>("enqueue_merge", { req }),
  cancelJob: (jobId) => invoke<void>("cancel_job", { jobId }),
  cancelAll: () => invoke<void>("cancel_all"),
  gpuCaps: () => invoke<GpuCaps>("gpu_caps"),

  listen: (event, handler) => listen(event, (e) => handler(e.payload as never)),

  // Tauri's *native* drag-drop (dragDropEnabled: true): WebView2 hands us absolute OS
  // paths, which ffmpeg/pandoc need — HTML5 drop would only give blobs.
  onFileDrop: ({ enter, leave, drop }) =>
    getCurrentWebview().onDragDropEvent(({ payload }) => {
      if (payload.type === "enter" && payload.paths.length > 0) enter(); // text/link drags carry no paths
      else if (payload.type === "leave") leave();
      else if (payload.type === "drop") drop(payload.paths);
    }),

  pickFiles: () => open({ multiple: true, directory: false }),
  // directory + multiple:false → string | null, already an absolute Windows path.
  pickOutputDir: () => open({ directory: true, multiple: false }),
  reveal: (output) => revealItemInDir(output),
  revealAll: async (outputs) => {
    if (outputs[0]) await revealItemInDir(outputs[0]);
  },
};

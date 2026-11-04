import type { FileInfo } from "../../types/generated/FileInfo";
import type { Format } from "../../types/generated/Format";
import type { Platform } from "../types";
import { emit, on } from "./bus";
import { fileFor, filesFromDrop, pickFiles, register } from "./files";
import { canPickFolder, download, downloadAll, pickFolder, store } from "./outputs";
import * as pool from "./pool";
import type { Mount } from "./protocol";

// Browser host (GitHub Pages build). Same contract as platform/tauri: string "paths"
// in, conversion://* events out. Paths are File-registry keys; outputs are Blob keys.

/** Probe results carry the formats Rust detected; jobs need them to skip re-detection. */
const detected = new Map<string, Format>();

function mountOf(key: string, dir: string): Mount {
  const file = fileFor(key);
  if (!file) throw new Error("file is no longer available — add it again");
  return { dir, file };
}

async function probeFiles(keys: string[]): Promise<FileInfo[]> {
  const mounts = keys.map((k, i) => mountOf(k, String(i)));
  const infos = await pool.probe(mounts);
  // Rust saw `/in/<i>/<name>`; hand the UI back the registry key it gave us.
  return infos.map((info) => {
    const i = Number(info.path.split("/")[2]);
    const key = keys[i] ?? info.path;
    if (info.format) detected.set(key, info.format);
    return { ...info, path: key };
  });
}

let dragDepth = 0;

export const platform: Platform = {
  kind: "web",
  hasWindowControls: false,
  revealLabel: "Download",
  revealAllLabel: "Download all",

  probeFiles,

  enqueueJobs: async (jobs) => {
    for (const job of jobs) {
      const src = detected.get(job.path);
      const finish = (name: string, blob: Blob) => store(job.jobId, name, blob, job.outputDir !== null);
      if (!src) {
        emit("conversion://error", { jobId: job.jobId, message: "unknown input format", detail: null, cancelled: false });
        continue;
      }
      pool.enqueue({ id: job.jobId, req: { kind: "convert", mount: mountOf(job.path, "0"), src, target: job.target, settings: job.settings }, finish });
    }
  },

  enqueueMerge: async (req) => {
    const mounts = req.paths.map((p, i) => mountOf(p, String(i)));
    const formats = req.paths.map((p) => detected.get(p) ?? "png");
    const finish = (name: string, blob: Blob) => store(req.jobId, name, blob, req.outputDir !== null);
    pool.enqueue({ id: req.jobId, req: { kind: "merge", mounts, formats, settings: req.settings }, finish });
  },

  cancelJob: async (id) => pool.cancel(id),
  cancelAll: async () => pool.cancelAll(),
  // No hardware encoders in a browser (yet): "ready" with none, so nothing says "detecting…".
  gpuCaps: async () => ({ ready: true, encoders: [] }),

  listen: async (event, handler) => on(event, handler),

  onFileDrop: async ({ enter, leave, drop }) => {
    const hasFiles = (e: DragEvent) => e.dataTransfer?.types.includes("Files") ?? false;
    // dragenter/leave fire for every child element crossed; a depth counter turns
    // that into one enter and one leave for the window as a whole.
    const onEnter = (e: DragEvent) => {
      if (hasFiles(e) && dragDepth++ === 0) enter();
    };
    const onLeave = (e: DragEvent) => {
      if (hasFiles(e) && --dragDepth === 0) leave();
    };
    const onOver = (e: DragEvent) => {
      if (hasFiles(e)) e.preventDefault(); // required, or the browser navigates to the file
    };
    const onDrop = (e: DragEvent) => {
      if (!e.dataTransfer || !hasFiles(e)) return;
      e.preventDefault();
      dragDepth = 0;
      leave();
      void filesFromDrop(e.dataTransfer).then((files) => drop(files.map(register)));
    };
    const pairs = [["dragenter", onEnter], ["dragleave", onLeave], ["dragover", onOver], ["drop", onDrop]] as const;
    for (const [type, fn] of pairs) window.addEventListener(type, fn);
    return () => pairs.forEach(([type, fn]) => window.removeEventListener(type, fn));
  },

  pickFiles: async () => (await pickFiles())?.map(register) ?? null,
  pickOutputDir: canPickFolder ? pickFolder : null,
  reveal: async (output) => download(output),
  revealAll: (outputs) => downloadAll(outputs),
};

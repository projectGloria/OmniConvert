import { open } from "@tauri-apps/plugin-dialog";
import { useApp } from "../store/appStore";
import { useQueue } from "../store/queueStore";
import { probeFiles } from "./ipc";

/**
 * The single entry point for "files enter the queue" — drop, Browse, Ctrl+O and the
 * header's Add button all land here, so feedback (duplicates, failures) is uniform.
 */
export async function ingest(paths: string[]): Promise<void> {
  if (paths.length === 0) return;
  const { notify } = useApp.getState();
  try {
    const files = await probeFiles(paths);
    const added = useQueue.getState().addFiles(files);
    const skipped = files.length - added;
    if (files.length === 0) notify("No files found in that selection");
    else if (skipped > 0) notify(`${skipped} ${skipped === 1 ? "file was" : "files were"} already in the queue`);
  } catch (err) {
    notify(`Couldn't read files: ${String(err)}`, "error");
  }
}

/** Native multi-file picker → ingest. */
export async function browse(): Promise<void> {
  const picked = await open({ multiple: true, directory: false });
  if (picked) await ingest(picked);
}

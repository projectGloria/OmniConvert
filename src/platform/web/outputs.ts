import { zip } from "fflate";

// File System Access (Chromium only) isn't in TypeScript's DOM lib yet.
declare global {
  interface Window {
    showDirectoryPicker(options?: { mode?: "read" | "readwrite" }): Promise<FileSystemDirectoryHandle>;
  }
}

// Finished outputs live here as Blobs; their "path" in the UI is an opaque key. If the
// user granted a folder (File System Access, Chromium only), each output is also
// written there as it finishes — the closest a browser gets to "save next to source".

const outputs = new Map<string, { name: string; blob: Blob }>();
let folder: FileSystemDirectoryHandle | null = null;

export const canPickFolder = typeof window !== "undefined" && "showDirectoryPicker" in window;

export async function pickFolder(): Promise<string | null> {
  try {
    const picked = await window.showDirectoryPicker({ mode: "readwrite" });
    folder = picked;
    return picked.name;
  } catch {
    return null; // AbortError = dismissed; SecurityError = no user gesture. Both: keep current.
  }
}

export async function store(jobId: string, name: string, blob: Blob, toFolder: boolean): Promise<string> {
  const key = `web-out:${jobId}/${name}`;
  let finalName = name;
  if (toFolder && folder) finalName = await writeUnique(folder, name, blob);
  outputs.set(key, { name: finalName, blob });
  return key;
}

/** `photo.webp` → `photo (1).webp`…, never overwrite (same rule as the desktop). */
async function writeUnique(dir: FileSystemDirectoryHandle, name: string, blob: Blob): Promise<string> {
  const dot = name.lastIndexOf(".");
  const [base, ext] = dot > 0 ? [name.slice(0, dot), name.slice(dot)] : [name, ""];
  for (let n = 0; n < 10_000; n++) {
    const candidate = n === 0 ? name : `${base} (${n})${ext}`;
    const exists = await dir.getFileHandle(candidate).then(() => true, () => false);
    if (exists) continue;
    const w = await (await dir.getFileHandle(candidate, { create: true })).createWritable();
    await w.write(blob);
    await w.close();
    return candidate;
  }
  throw new Error("too many files with this name");
}

function save(name: string, blob: Blob): void {
  const url = URL.createObjectURL(blob);
  const a = Object.assign(document.createElement("a"), { href: url, download: name });
  a.click();
  // Revoke later: revoking synchronously can cancel the download in Firefox.
  setTimeout(() => URL.revokeObjectURL(url), 60_000);
}

export function download(key: string): void {
  const o = outputs.get(key);
  if (o) save(o.name, o.blob);
}

/** One file → plain download. Several → one ZIP (browsers block bursts of downloads). */
export async function downloadAll(keys: string[]): Promise<void> {
  const found = keys.map((k) => outputs.get(k)).filter((o) => o !== undefined);
  if (found.length === 1 && found[0]) return save(found[0].name, found[0].blob);
  if (found.length === 0) return;

  const files: Record<string, [Uint8Array, { level: 0 }]> = {};
  for (const o of found) {
    let name = o.name;
    for (let n = 1; name in files; n++) name = o.name.replace(/(\.[^.]+)?$/, ` (${n})$1`);
    // level 0 = store: outputs are mostly already compressed (JPG, WebP, XLSX, 7z…).
    files[name] = [new Uint8Array(await o.blob.arrayBuffer()), { level: 0 }];
  }
  // Async zip runs off the main thread, so a big batch doesn't freeze the UI.
  const bytes = await new Promise<Uint8Array>((res, rej) => zip(files, (err, data) => (err ? rej(err) : res(data))));
  save("omniconvert-outputs.zip", new Blob([bytes as Uint8Array<ArrayBuffer>], { type: "application/zip" }));
}

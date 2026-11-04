// Browser files have no paths. The queue (and the whole UI contract) is keyed by path
// strings, so each File gets a stable synthetic key. Name + size + mtime makes dropping
// the same file twice dedupe exactly like the desktop's path check does.

const registry = new Map<string, File>();

export function register(file: File): string {
  const key = `web:${file.webkitRelativePath || file.name}|${file.size}|${file.lastModified}`;
  registry.set(key, file);
  return key;
}

export const fileFor = (key: string): File | undefined => registry.get(key);

/** Hidden <input type=file>. Resolves null on dismiss (the `cancel` event). */
export function pickFiles(): Promise<File[] | null> {
  return new Promise((resolve) => {
    const input = document.createElement("input");
    input.type = "file";
    input.multiple = true;
    input.onchange = () => resolve(input.files ? [...input.files] : null);
    input.oncancel = () => resolve(null);
    input.click();
  });
}

/** Files from a drop, recursing into dropped folders (like desktop's probe_files does). */
export async function filesFromDrop(dt: DataTransfer): Promise<File[]> {
  // Entries must be grabbed synchronously: DataTransfer is emptied after the event tick.
  const entries = [...dt.items].map((i) => i.webkitGetAsEntry()).filter((e): e is FileSystemEntry => e !== null);
  if (entries.length === 0) return [...dt.files];
  const nested = await Promise.all(entries.map(walk));
  return nested.flat();
}

async function walk(entry: FileSystemEntry): Promise<File[]> {
  if (entry.isFile) {
    return new Promise((res) => (entry as FileSystemFileEntry).file((f) => res([f]), () => res([])));
  }
  const reader = (entry as FileSystemDirectoryEntry).createReader();
  const children: FileSystemEntry[] = [];
  // readEntries returns batches (Chromium: 100 at a time) until an empty batch.
  for (;;) {
    const batch = await new Promise<FileSystemEntry[]>((res) => reader.readEntries(res, () => res([])));
    if (batch.length === 0) break;
    children.push(...batch);
  }
  return (await Promise.all(children.map(walk))).flat();
}

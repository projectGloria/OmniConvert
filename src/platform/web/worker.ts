import { ConsoleStdout, Directory, File as WasiFile, type Inode, OpenFile, PreopenDirectory, WASI } from "@bjorn3/browser_wasi_shim";
import type { Format } from "../../types/generated/Format";
import type { Mount, WorkerReply, WorkerRequest } from "./protocol";
import { BlobFile, patchGrowableWrites } from "./wasiFs";

// One worker = one job. The pool terminates the worker on cancel, which drops the WASM
// instance mid-instruction: the browser equivalent of the desktop's Job Object kill.

// Typed against the DOM lib (the project's single lib set): pulling in lib.webworker
// alongside DOM produces duplicate-declaration errors. The APIs used here exist in both.
patchGrowableWrites();

const post = (msg: WorkerReply) => self.postMessage(msg);
const progress = (value: number | null, stage: string) => post({ type: "progress", progress: value, stage });

interface RunResult {
  code: number;
  reply: { output?: string; message?: string; detail?: string | null };
  out: Map<string, Inode>;
}

/** One WASI command run: inputs mounted lazily at /in, outputs collected from /out. */
async function run(module: WebAssembly.Module, cmd: string, body: unknown, mounts: Mount[], scale = [0, 1]): Promise<RunResult> {
  const inRoot = new Map<string, Inode>();
  for (const { dir, file } of mounts) {
    const d = (inRoot.get(dir) as Directory | undefined) ?? new Directory(new Map());
    d.contents.set(file.name, new BlobFile(file));
    inRoot.set(dir, d);
  }
  const out = new Map<string, Inode>();
  let stdout = "";
  const [from = 0, to = 1] = scale;
  const fds = [
    new OpenFile(new WasiFile([])),
    ConsoleStdout.lineBuffered((line) => (stdout += line)),
    // Progress lines from crates/web/src/sink.rs. Non-JSON lines (a Rust panic message)
    // are ignored here; the trap that follows surfaces as the job's error.
    ConsoleStdout.lineBuffered((line) => {
      try {
        const p = JSON.parse(line) as { progress: number | null; stage: string };
        progress(p.progress === null ? null : from + p.progress * (to - from), p.stage);
      } catch {
        /* not a progress line */
      }
    }),
    new PreopenDirectory("/in", inRoot),
    new PreopenDirectory("/out", out),
  ];
  const wasi = new WASI(["omniconvert-web", cmd, JSON.stringify(body)], [], fds);
  const instance = await WebAssembly.instantiate(module, { wasi_snapshot_preview1: wasi.wasiImport });
  const code = wasi.start(instance as unknown as Parameters<WASI["start"]>[0]);
  return { code, reply: stdout ? JSON.parse(stdout) : {}, out };
}

function outputOf(r: RunResult): { name: string; blob: Blob } {
  if (r.code !== 0) throw Object.assign(new Error(r.reply.message ?? `engine exited with ${r.code}`), { detail: r.reply.detail ?? null });
  const name = (r.reply.output ?? "").split("/").pop() ?? "";
  const file = r.out.get(name);
  if (!(file instanceof WasiFile)) throw new Error("engine reported an output it didn't write");
  return { name, blob: new Blob([file.data as Uint8Array<ArrayBuffer>]) };
}

/** Re-encode through the browser's own codecs (OffscreenCanvas). null if unsupported. */
async function browserEncode(blob: Blob, type: string, quality?: number): Promise<Blob | null> {
  const bmp = await createImageBitmap(blob, { premultiplyAlpha: "none", colorSpaceConversion: "none" });
  const canvas = new OffscreenCanvas(bmp.width, bmp.height);
  canvas.getContext("2d")?.drawImage(bmp, 0, 0);
  bmp.close();
  const out = await canvas.convertToBlob(quality === undefined ? { type } : { type, quality });
  // Safari silently falls back to PNG for formats it can't encode: check, don't trust.
  return out.type === type ? out : null;
}

const stem = (name: string) => name.replace(/\.[^.]+$/, "");

async function convert(req: Extract<WorkerRequest, { kind: "convert" }>) {
  let { mount, src } = req;
  // AVIF input: the desktop pipes it through ffmpeg; every modern browser decodes AVIF natively.
  if (src === "avif") {
    progress(null, "Decoding AVIF");
    const png = await browserEncode(mount.file, "image/png");
    if (!png) throw new Error("this browser can't decode AVIF");
    mount = { dir: mount.dir, file: new File([png], `${stem(mount.file.name)}.png`) };
    src = "png" satisfies Format;
  }

  const body = (target: typeof req.target) => ({ input: `/in/${mount.dir}/${mount.file.name}`, src, target, outDir: "/out", settings: req.settings });
  const quality = req.settings.quality ?? 85;
  const lossyWebp = req.target.type === "convert" && req.target.format === "webp" && quality < 100;

  if (lossyWebp) {
    // No libwebp in the WASM build: let Rust do decode/resize/flatten → PNG, then encode
    // lossy WebP with the browser. Falls back to Rust's lossless encoder (Safari).
    const png = outputOf(await run(req.module, "convert", body({ type: "convert", format: "png" }), [mount], [0, 0.8]));
    progress(0.85, "Encoding WebP");
    const webp = await browserEncode(png.blob, "image/webp", quality / 100);
    if (webp) return { name: `${stem(png.name)}.webp`, blob: webp };
  }
  return outputOf(await run(req.module, "convert", body(req.target), [mount]));
}

self.onmessage = async ({ data: req }: MessageEvent<WorkerRequest>) => {
  try {
    if (req.kind === "probe") {
      const r = await run(req.module, "probe", { paths: req.mounts.map((m) => `/in/${m.dir}/${m.file.name}`) }, req.mounts);
      post({ type: "probed", files: r.reply as unknown[] });
    } else if (req.kind === "merge") {
      const inputs = req.mounts.map((m, i) => [`/in/${m.dir}/${m.file.name}`, req.formats[i]]);
      post({ type: "done", ...outputOf(await run(req.module, "merge", { inputs, outDir: "/out", settings: req.settings }, req.mounts)) });
    } else {
      post({ type: "done", ...(await convert(req)) });
    }
  } catch (e) {
    // A WebAssembly.RuntimeError here is a Rust panic (panic=abort → trap).
    const crashed = e instanceof WebAssembly.RuntimeError;
    const message = crashed ? "converter crashed on this file" : e instanceof Error ? e.message : String(e);
    const detail = crashed ? String(e) : ((e as { detail?: string | null }).detail ?? null);
    post({ type: "error", message, detail });
  }
};

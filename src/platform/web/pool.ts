import wasmUrl from "./engine/omniconvert-web.wasm?url";
import type { FileInfo } from "../../types/generated/FileInfo";
import { emit } from "./bus";
import type { Mount, WorkerReply, WorkerRequest } from "./protocol";

// The browser's stand-in for the Rust JobQueue: a FIFO + a concurrency cap, one
// short-lived Worker per job. Fresh worker per job means cancel = terminate() with
// nothing to clean up, and a crashed (trapped) engine can't poison the next job.

/** Half the cores, capped: each job is one single-threaded WASM instance plus its
 *  in-memory output, and the UI thread needs headroom (same reasoning as the desktop's cores/2 pool). */
const LIMIT = Math.min(4, Math.max(1, Math.floor((navigator.hardwareConcurrency || 4) / 2)));

let compiled: Promise<WebAssembly.Module> | null = null;
/** Compile once; the Module is structured-cloned into each worker without recompiling. */
function engine(): Promise<WebAssembly.Module> {
  compiled ??= WebAssembly.compileStreaming(fetch(wasmUrl)).catch(async () =>
    // compileStreaming needs Content-Type: application/wasm; fall back for odd hosts.
    WebAssembly.compile(await (await fetch(wasmUrl)).arrayBuffer()),
  );
  // A failed fetch (offline before first load) must be retryable, not cached forever.
  compiled.catch(() => (compiled = null));
  return compiled;
}

const spawn = () => new Worker(new URL("./worker.ts", import.meta.url), { type: "module" });

type Request = WorkerRequest extends infer R ? (R extends WorkerRequest ? Omit<R, "module"> : never) : never;

interface Job {
  id: string;
  req: Request;
  /** Called with the finished output; resolves to its key for the complete event. */
  finish: (name: string, blob: Blob) => Promise<string>;
}

const waiting: Job[] = [];
const running = new Map<string, Worker>();

export function enqueue(job: Job): void {
  waiting.push(job);
  pump();
}

export function cancel(id: string): void {
  const i = waiting.findIndex((j) => j.id === id);
  if (i >= 0) waiting.splice(i, 1);
  const worker = running.get(id);
  worker?.terminate();
  running.delete(id);
  if (i >= 0 || worker) {
    emit("conversion://error", { jobId: id, message: "cancelled", detail: null, cancelled: true });
    pump();
  }
}

export const cancelAll = () => [...waiting.map((j) => j.id), ...running.keys()].forEach(cancel);

function pump(): void {
  while (running.size < LIMIT && waiting.length > 0) void start(waiting.shift() as Job);
}

async function start(job: Job): Promise<void> {
  const worker = spawn();
  running.set(job.id, worker);
  const began = performance.now();
  const end = (payload: Record<string, unknown> | null, error?: { message: string; detail: string | null }) => {
    if (running.get(job.id) !== worker) return; // cancelled meanwhile: cancel() already reported
    running.delete(job.id);
    worker.terminate();
    if (error) emit("conversion://error", { jobId: job.id, ...error, cancelled: false });
    else emit("conversion://complete", { jobId: job.id, ...payload });
    pump();
  };

  worker.onmessage = async ({ data }: MessageEvent<WorkerReply>) => {
    if (data.type === "progress") emit("conversion://progress", { jobId: job.id, progress: data.progress, stage: data.stage });
    else if (data.type === "error") end(null, data);
    else if (data.type === "done") {
      try {
        const output = await job.finish(data.name, data.blob);
        end({ output, outputBytes: data.blob.size, elapsedMs: Math.round(performance.now() - began) });
      } catch (e) {
        end(null, { message: `couldn't save output: ${String(e)}`, detail: null });
      }
    }
  };
  worker.onerror = (e) => end(null, { message: "converter crashed", detail: e.message || null });

  try {
    worker.postMessage({ ...job.req, module: await engine() } as WorkerRequest);
  } catch (e) {
    end(null, { message: "couldn't load the conversion engine", detail: String(e) });
  }
}

/** Probe outside the job queue: it's milliseconds of work and must never wait behind a long encode. */
export async function probe(mounts: Mount[]): Promise<FileInfo[]> {
  const module = await engine();
  const worker = spawn();
  try {
    return await new Promise<FileInfo[]>((resolve, reject) => {
      worker.onmessage = ({ data }: MessageEvent<WorkerReply>) => {
        if (data.type === "probed") resolve(data.files as FileInfo[]);
        else if (data.type === "error") reject(new Error(data.message));
      };
      worker.onerror = (e) => reject(new Error(e.message));
      worker.postMessage({ kind: "probe", module, mounts } satisfies WorkerRequest);
    });
  } finally {
    worker.terminate();
  }
}

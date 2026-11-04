import type { FileInfo } from "../types/generated/FileInfo";
import type { GpuCaps } from "../types/generated/GpuCaps";
import type { JobRequest } from "../types/generated/JobRequest";
import type { MergeRequest } from "../types/generated/MergeRequest";

export type Unlisten = () => void;

export interface DropHandlers {
  enter: () => void;
  leave: () => void;
  /** Opaque source ids: OS paths on desktop, registered File keys on the web. */
  drop: (paths: string[]) => void;
}

/**
 * Everything the UI needs from its host. The React tree is identical on both builds;
 * only this object differs (`@platform` resolves to platform/tauri or platform/web at
 * build time — see vite.config.ts), so the web bundle never ships Tauri imports and
 * vice versa.
 *
 * The contract is the desktop's: "paths" are strings, results come back as
 * conversion://progress|complete|error events. The web adapter fakes paths with keys
 * into its File registry and emits the same events from its worker pool.
 */
export interface Platform {
  readonly kind: "desktop" | "web";
  /** Frameless window: we draw minimize/maximize/close. False in a browser tab. */
  readonly hasWindowControls: boolean;
  /** Wording for the per-row "where's my file" action. */
  readonly revealLabel: string;
  /** Wording for the batch summary's bulk action. */
  readonly revealAllLabel: string;

  probeFiles(paths: string[]): Promise<FileInfo[]>;
  enqueueJobs(jobs: JobRequest[]): Promise<void>;
  enqueueMerge(req: MergeRequest): Promise<void>;
  cancelJob(jobId: string): Promise<void>;
  cancelAll(): Promise<void>;
  gpuCaps(): Promise<GpuCaps>;

  /** Payload-only listener for backend events (conversion://*, system://gpu-ready). */
  listen<T>(event: string, handler: (payload: T) => void): Promise<Unlisten>;
  onFileDrop(handlers: DropHandlers): Promise<Unlisten>;
  /** Native/HTML file picker. null = user dismissed it. */
  pickFiles(): Promise<string[] | null>;
  /** null when this host can't target a folder (Firefox/Safari): the button is hidden. */
  readonly pickOutputDir: (() => Promise<string | null>) | null;
  /** Desktop: select in Explorer. Web: download. */
  reveal(output: string): Promise<void>;
  /** Desktop: open the first output's folder. Web: download everything (ZIP if > 1). */
  revealAll(outputs: string[]): Promise<void>;
}

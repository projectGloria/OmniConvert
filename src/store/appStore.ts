import { create } from "zustand";
import type { GpuCaps } from "../types/generated/GpuCaps";

export interface Toast {
  /** Monotonic: the Toast component re-arms its timer per id, even for identical text. */
  id: number;
  text: string;
  tone: "info" | "error";
}

interface AppState {
  /** null = write each output next to its source file. */
  outputDir: string | null;
  setOutputDir: (dir: string | null) => void;
  /** `ready: false` until the startup probe reports via system://gpu-ready. */
  gpu: GpuCaps;
  setGpu: (gpu: GpuCaps) => void;
  toast: Toast | null;
  notify: (text: string, tone?: Toast["tone"]) => void;
  dismissToast: (id: number) => void;
}

let toastSeq = 0;

export const useApp = create<AppState>()((set) => ({
  outputDir: null,
  setOutputDir: (outputDir) => set({ outputDir }),
  gpu: { ready: false, encoders: [] },
  setGpu: (gpu) => set({ gpu }),
  toast: null,
  notify: (text, tone = "info") => set({ toast: { id: ++toastSeq, text, tone } }),
  // Id-guarded: a stale timer from toast #3 must not dismiss toast #4.
  dismissToast: (id) => set((s) => (s.toast?.id === id ? { toast: null } : s)),
}));

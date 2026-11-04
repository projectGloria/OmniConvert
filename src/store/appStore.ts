import { create } from "zustand";

interface AppState {
  /** null = write each output next to its source file. */
  outputDir: string | null;
  setOutputDir: (dir: string | null) => void;
}

export const useApp = create<AppState>()((set) => ({
  outputDir: null,
  setOutputDir: (outputDir) => set({ outputDir }),
}));

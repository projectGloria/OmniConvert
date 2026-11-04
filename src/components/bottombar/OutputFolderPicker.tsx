import { Download, FolderOpen, X } from "lucide-react";
import { platform } from "@platform";
import { shortPath } from "../../lib/format";
import { useApp } from "../../store/appStore";

/** Where outputs land when no folder is chosen. Browsers can't write next to the source. */
const DEFAULT_LABEL = platform.kind === "web" ? "Downloads" : "Same as source";
const DEFAULT_HINT = platform.kind === "web" ? "Each output gets a Download button" : "Outputs are written next to each source file";

export function OutputFolderPicker() {
  const outputDir = useApp((s) => s.outputDir);
  const setOutputDir = useApp((s) => s.setOutputDir);
  const pickOutputDir = platform.pickOutputDir;

  // Firefox/Safari have no folder access at all: show where files go, not a dead button.
  if (!pickOutputDir) {
    return (
      <div title={DEFAULT_HINT} className="flex min-w-0 items-center gap-2 px-2 py-1.5 text-sm">
        <Download size={16} strokeWidth={1.75} className="shrink-0 text-muted" />
        <span className="text-muted">Output:</span>
        <span className="truncate text-ink">{DEFAULT_LABEL}</span>
      </div>
    );
  }

  const pick = async () => {
    const dir = await pickOutputDir();
    if (dir) setOutputDir(dir);
  };

  return (
    <div className="flex min-w-0 items-center gap-1">
      <button
        type="button"
        onClick={() => void pick()}
        title={outputDir ?? DEFAULT_HINT}
        className="flex min-w-0 items-center gap-2 rounded-md px-2 py-1.5 text-sm transition-colors hover:bg-raised"
      >
        <FolderOpen size={16} strokeWidth={1.75} className="shrink-0 text-muted" />
        <span className="text-muted">Output:</span>
        <span className="truncate text-ink">{outputDir ? shortPath(outputDir) : DEFAULT_LABEL}</span>
      </button>
      {outputDir && (
        <button
          type="button"
          aria-label="Reset output folder"
          onClick={() => setOutputDir(null)}
          className="grid size-6 place-items-center rounded text-muted hover:bg-raised hover:text-ink"
        >
          <X size={13} />
        </button>
      )}
    </div>
  );
}

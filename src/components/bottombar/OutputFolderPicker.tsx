import { open } from "@tauri-apps/plugin-dialog";
import { FolderOpen, X } from "lucide-react";
import { shortPath } from "../../lib/format";
import { useApp } from "../../store/appStore";

export function OutputFolderPicker() {
  const outputDir = useApp((s) => s.outputDir);
  const setOutputDir = useApp((s) => s.setOutputDir);

  const pick = async () => {
    // directory + multiple:false → string | null, already an absolute Windows path.
    const dir = await open({ directory: true, multiple: false });
    if (dir) setOutputDir(dir);
  };

  return (
    <div className="flex min-w-0 items-center gap-1">
      <button
        type="button"
        onClick={() => void pick()}
        title={outputDir ?? "Outputs are written next to each source file"}
        className="flex min-w-0 items-center gap-2 rounded-md px-2 py-1.5 text-sm transition-colors hover:bg-raised"
      >
        <FolderOpen size={16} strokeWidth={1.75} className="shrink-0 text-muted" />
        <span className="text-muted">Output:</span>
        <span className="truncate text-ink">{outputDir ? shortPath(outputDir) : "Same as source"}</span>
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

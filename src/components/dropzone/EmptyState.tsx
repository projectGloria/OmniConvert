import { open } from "@tauri-apps/plugin-dialog";
import { FileStack } from "lucide-react";
import { probeFiles } from "../../lib/ipc";
import { useQueue } from "../../store/queueStore";

const DOMAINS = ["Video", "Audio", "Image", "Document", "Data", "Archive"] as const;

export function EmptyState() {
  const addFiles = useQueue((s) => s.addFiles);

  // Fallback for users who don't drag-drop. Same Rust probe path as a drop,
  // so there's exactly one code path for "files enter the queue".
  const browse = async () => {
    const picked = await open({ multiple: true, directory: false });
    if (!picked || picked.length === 0) return;
    try {
      addFiles(await probeFiles(picked));
    } catch (err) {
      console.error("probe_files failed", err);
    }
  };

  return (
    <div className="grid h-full place-items-center p-6">
      <div className="flex w-full max-w-md flex-col items-center gap-5 rounded-2xl border border-dashed border-line px-8 py-12 text-center">
        <div className="grid size-14 place-items-center rounded-2xl border border-line bg-surface text-muted">
          <FileStack size={26} strokeWidth={1.5} />
        </div>
        <div className="space-y-1.5">
          <h1 className="text-base font-semibold text-ink">Drop files anywhere</h1>
          <p className="text-sm text-muted">Everything converts locally. Nothing leaves this machine.</p>
        </div>
        <button
          type="button"
          onClick={() => void browse()}
          className="rounded-lg border border-line bg-surface px-4 py-2 text-sm text-ink transition-colors hover:border-accent/60 hover:bg-raised"
        >
          Browse files
        </button>
        <ul className="flex flex-wrap justify-center gap-1.5">
          {DOMAINS.map((d) => (
            <li key={d} className="rounded-full border border-line px-2.5 py-0.5 text-[11px] text-muted">
              {d}
            </li>
          ))}
        </ul>
      </div>
    </div>
  );
}

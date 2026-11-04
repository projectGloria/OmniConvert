import { FileStack } from "lucide-react";
import { platform } from "@platform";
import { browse } from "../../lib/ingest";

// The web build runs only the pure-Rust engines (for now); don't advertise the rest.
const DOMAINS = platform.kind === "web" ? (["Image", "Spreadsheet", "Data", "Archive"] as const) : (["Video", "Audio", "Image", "Document", "Data", "Archive"] as const);
const PRIVACY =
  platform.kind === "web"
    ? "Everything converts in your browser. Nothing is uploaded."
    : "Everything converts locally. Nothing leaves this machine.";

export function EmptyState() {
  // Fallback for users who don't drag-drop. Same ingest() path as a drop and Ctrl+O.
  return (
    <div className="grid h-full place-items-center p-6">
      <div className="flex w-full max-w-md flex-col items-center gap-5 rounded-2xl border border-dashed border-line px-8 py-12 text-center">
        <div className="grid size-14 place-items-center rounded-2xl border border-line bg-surface text-muted">
          <FileStack size={26} strokeWidth={1.5} />
        </div>
        <div className="space-y-1.5">
          <h1 className="text-base font-semibold text-ink">Drop files anywhere</h1>
          <p className="text-sm text-muted">{PRIVACY}</p>
        </div>
        <button
          type="button"
          onClick={() => void browse()}
          className="rounded-lg border border-line bg-surface px-4 py-2 text-sm text-ink transition-colors hover:border-accent/60 hover:bg-raised"
        >
          Browse files
        </button>
        <p className="text-[11px] text-muted/70">
          <kbd className="font-sans">Ctrl+O</kbd> add · <kbd className="font-sans">Ctrl+Enter</kbd> convert
        </p>
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

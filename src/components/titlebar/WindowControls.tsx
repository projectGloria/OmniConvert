import { useEffect, useState, type ReactNode } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { Copy, Minus, Square, X } from "lucide-react";
import { cn } from "../../lib/cn";

export function WindowControls() {
  const [maximized, setMaximized] = useState(false);

  // Window state lives outside React (OS-owned), so an effect subscription is the
  // correct tool here — this is syncing *from* an external system, not deriving state.
  useEffect(() => {
    const win = getCurrentWindow();
    let disposed = false;
    let unlisten: (() => void) | undefined;

    const sync = () => {
      void win.isMaximized().then((v) => !disposed && setMaximized(v));
    };
    sync();
    // Maximize via Win+Up, double-click, or Aero Snap all surface as a resize.
    win.onResized(sync).then((fn) => (disposed ? fn() : (unlisten = fn)));

    return () => {
      disposed = true;
      unlisten?.();
    };
  }, []);

  return (
    <div className="flex h-full">
      <ControlButton label="Minimize" onClick={() => void getCurrentWindow().minimize()}>
        <Minus size={14} strokeWidth={1.5} />
      </ControlButton>
      <ControlButton
        label={maximized ? "Restore" : "Maximize"}
        onClick={() => void getCurrentWindow().toggleMaximize()}
      >
        {/* Copy = two stacked squares, the conventional "restore" glyph. */}
        {maximized ? <Copy size={12} strokeWidth={1.5} className="-scale-x-100" /> : <Square size={11} strokeWidth={1.5} />}
      </ControlButton>
      <ControlButton label="Close" danger onClick={() => void getCurrentWindow().close()}>
        <X size={15} strokeWidth={1.5} />
      </ControlButton>
    </div>
  );
}

interface ControlButtonProps {
  label: string;
  onClick: () => void;
  danger?: boolean;
  children: ReactNode;
}

function ControlButton({ label, onClick, danger = false, children }: ControlButtonProps) {
  return (
    <button
      type="button"
      aria-label={label}
      title={label}
      onClick={onClick}
      tabIndex={-1} // window chrome shouldn't steal Tab focus from the queue
      className={cn(
        "grid h-full w-[46px] place-items-center text-muted transition-colors duration-100",
        danger ? "hover:bg-[#c42b1c] hover:text-white" : "hover:bg-raised hover:text-ink",
      )}
    >
      {children}
    </button>
  );
}

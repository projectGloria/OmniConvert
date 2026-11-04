import { lazy, Suspense } from "react";
import { platform } from "@platform";

// Lazy: the web build never mounts this, so its @tauri-apps/api/window chunk is never fetched.
const WindowControls = lazy(() => import("./WindowControls").then((m) => ({ default: m.WindowControls })));

/**
 * Custom titlebar for the frameless window (desktop) / app header (web).
 *
 * `data-tauri-drag-region` is checked on the *event target*, not ancestors — so the
 * attribute must sit on every element the user might grab (bar, logo, title text),
 * otherwise mousedown on the title span wouldn't drag. Interactive controls must NOT
 * carry it, or clicks become drags. Double-click on a drag region toggles maximize natively.
 * In a browser the attribute is inert.
 */
export function Titlebar() {
  return (
    <header
      data-tauri-drag-region
      className="flex h-9 shrink-0 items-center justify-between border-b border-line bg-base"
    >
      <div data-tauri-drag-region className="flex items-center gap-2 pl-3">
        {/* BASE_URL: the Pages build is served from /OmniConvert/, not the domain root. */}
        <img data-tauri-drag-region src={`${import.meta.env.BASE_URL}app-icon.svg`} alt="" className="size-4" draggable={false} />
        <span data-tauri-drag-region className="text-xs font-medium tracking-wide text-muted">
          OmniConvert
        </span>
      </div>
      {platform.hasWindowControls && (
        <Suspense fallback={null}>
          <WindowControls />
        </Suspense>
      )}
    </header>
  );
}

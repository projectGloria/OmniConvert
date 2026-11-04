import { useState } from "react";
import {
  FloatingFocusManager,
  FloatingPortal,
  autoUpdate,
  flip,
  offset,
  shift,
  useClick,
  useDismiss,
  useFloating,
  useInteractions,
  useRole,
} from "@floating-ui/react";
import { Settings2 } from "lucide-react";
import { cn } from "../../lib/cn";
import { fieldsFor, hasOverrides } from "../../lib/settingsSchema";
import { ACTIVE, type QueueItem } from "../../types/queue";
import { SettingsPanel } from "./SettingsPanel";

/**
 * Per-row gear. Renders nothing when the (source, target) pair has no knobs — data,
 * document and archive rows stay visually clean (CLAUDE.md "Zero Clutter").
 */
export function SettingsButton({ item }: { item: QueueItem }) {
  const [open, setOpen] = useState(false);
  const target = item.targets.find((t) => t.id === item.target);
  // Derived per render from (source, target): there's no stored "has settings" flag to go stale.
  const fields = fieldsFor(item, target);
  const locked = ACTIVE.has(item.status);

  const { refs, floatingStyles, context } = useFloating({
    open: open && !locked,
    onOpenChange: setOpen,
    placement: "bottom-end",
    whileElementsMounted: autoUpdate,
    middleware: [offset(6), flip({ padding: 8 }), shift({ padding: 8 })],
  });
  const { getReferenceProps, getFloatingProps } = useInteractions([
    useClick(context),
    useDismiss(context),
    useRole(context, { role: "dialog" }),
  ]);

  if (fields.length === 0) return null;
  const custom = hasOverrides(item.settings, fields);

  return (
    <>
      <button
        ref={refs.setReference}
        type="button"
        disabled={locked}
        aria-label="Conversion settings"
        title={locked ? "Settings are locked while converting" : custom ? "Custom settings" : "Settings"}
        {...getReferenceProps()}
        className={cn(
          "relative grid size-7 place-items-center rounded-md transition",
          open ? "bg-raised text-ink" : "text-muted hover:bg-raised hover:text-ink",
          "disabled:cursor-not-allowed disabled:opacity-40 disabled:hover:bg-transparent",
        )}
      >
        <Settings2 size={15} strokeWidth={1.75} />
        {/* Dot = "this row deviates from defaults" — visible without opening anything. */}
        {custom && <span className="absolute top-1 right-1 size-1.5 rounded-full bg-accent" />}
      </button>

      {open && !locked && (
        <FloatingPortal>
          {/* modal={false}: Tab walks the controls and out; focus returns to the gear on close. */}
          <FloatingFocusManager context={context} modal={false}>
            <div ref={refs.setFloating} style={floatingStyles} {...getFloatingProps()} className="z-50">
              <SettingsPanel item={item} target={target} fields={fields} />
            </div>
          </FloatingFocusManager>
        </FloatingPortal>
      )}
    </>
  );
}

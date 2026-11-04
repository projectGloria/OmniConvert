import { useState } from "react";
import {
  FloatingFocusManager,
  FloatingPortal,
  autoUpdate,
  flip,
  offset,
  shift,
  size,
  useClick,
  useDismiss,
  useFloating,
  useInteractions,
  useRole,
} from "@floating-ui/react";
import { ChevronDown } from "lucide-react";
import { cn } from "../../lib/cn";
import type { Target } from "../../types/generated/Target";
import { FormatPopover } from "./FormatPopover";

interface FormatPillProps {
  targets: Target[];
  selectedId: string | null;
  onSelect: (targetId: string) => void;
}

export function FormatPill({ targets, selectedId, onSelect }: FormatPillProps) {
  const [open, setOpen] = useState(false);
  // Derived on render: no need to store the selected Target object in state.
  const selected = selectedId ? targets.find((t) => t.id === selectedId) : undefined;
  const disabled = targets.length === 0;

  const { refs, floatingStyles, context } = useFloating({
    open,
    onOpenChange: setOpen,
    placement: "bottom-end",
    // autoUpdate re-positions on scroll/resize while open — the queue list scrolls.
    whileElementsMounted: autoUpdate,
    middleware: [
      offset(6),
      flip({ padding: 8 }),
      shift({ padding: 8 }),
      // Cap height to the space actually available so long lists scroll instead of clipping.
      size({
        padding: 8,
        apply({ availableHeight, elements }) {
          elements.floating.style.maxHeight = `${Math.min(380, availableHeight)}px`;
        },
      }),
    ],
  });

  const { getReferenceProps, getFloatingProps } = useInteractions([
    useClick(context),
    useDismiss(context), // Esc + outside click
    useRole(context, { role: "listbox" }),
  ]);

  return (
    <>
      <button
        ref={refs.setReference}
        type="button"
        disabled={disabled}
        {...getReferenceProps()}
        className={cn(
          "flex min-w-32 items-center justify-between gap-2 rounded-md border px-2.5 py-1 text-xs transition-colors",
          "disabled:cursor-not-allowed disabled:opacity-60",
          selected
            ? "border-accent/50 bg-accent/10 text-ink hover:bg-accent/15"
            : "border-line bg-base text-muted hover:border-[#2a2a32] hover:text-ink",
        )}
      >
        <span className="truncate">{disabled ? "Unsupported" : (selected?.label ?? "Choose format")}</span>
        {!disabled && <ChevronDown size={13} className={cn("shrink-0 transition-transform", open && "rotate-180")} />}
      </button>

      {open && (
        <FloatingPortal>
          {/* modal={false}: no focus trap, Tab-out just closes. Focus moves to the first option on open. */}
          <FloatingFocusManager context={context} modal={false}>
            <div ref={refs.setFloating} style={floatingStyles} {...getFloatingProps()} className="z-50">
              <FormatPopover
                targets={targets}
                selectedId={selectedId}
                onSelect={(id) => {
                  onSelect(id);
                  setOpen(false);
                }}
              />
            </div>
          </FloatingFocusManager>
        </FloatingPortal>
      )}
    </>
  );
}

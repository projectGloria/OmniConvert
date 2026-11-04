import { Check } from "lucide-react";
import { CATEGORY_LABEL, CATEGORY_TINT, groupTargets } from "../../lib/categories";
import { cn } from "../../lib/cn";
import type { Target } from "../../types/generated/Target";

interface FormatPopoverProps {
  targets: Target[];
  selectedId: string | null;
  onSelect: (targetId: string) => void;
}

/**
 * Grouped target list. Phase 4 adds the search field, arrow-key navigation
 * and "Apply to all" footer on top of this same component.
 */
export function FormatPopover({ targets, selectedId, onSelect }: FormatPopoverProps) {
  // Cheap (≤ ~20 targets), so recomputing per render beats a useMemo's bookkeeping.
  const groups = groupTargets(targets);

  return (
    <div
      // overflow-y on the inner box so the size() middleware's maxHeight applies here.
      className="flex max-h-[inherit] w-64 flex-col overflow-y-auto rounded-lg border border-line bg-surface p-1 shadow-2xl shadow-black/60"
    >
      {groups.map(({ group, targets: items }) => (
        <section key={group} className="py-1">
          <h3 className="flex items-center gap-1.5 px-2 pt-1 pb-1.5 text-[10px] font-semibold tracking-wider text-muted uppercase">
            <span className="size-1.5 rounded-full" style={{ backgroundColor: CATEGORY_TINT[group] }} />
            {CATEGORY_LABEL[group]}
          </h3>
          <ul>
            {items.map((t) => {
              const active = t.id === selectedId;
              return (
                <li key={t.id}>
                  <button
                    type="button"
                    role="option"
                    aria-selected={active}
                    onClick={() => onSelect(t.id)}
                    className={cn(
                      "flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-left text-sm transition-colors",
                      active ? "bg-accent/15 text-ink" : "text-ink/90 hover:bg-raised",
                    )}
                  >
                    <span className="min-w-0 flex-1">
                      <span className="block truncate">{t.label}</span>
                      {t.note && <span className="block truncate text-[11px] text-muted">{t.note}</span>}
                    </span>
                    {active && <Check size={14} className="shrink-0 text-accent" />}
                  </button>
                </li>
              );
            })}
          </ul>
        </section>
      ))}
    </div>
  );
}

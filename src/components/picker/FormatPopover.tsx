import { useEffect, useId, useState, type KeyboardEvent } from "react";
import { Check, Search } from "lucide-react";
import { CATEGORY_LABEL, CATEGORY_TINT, groupTargets } from "../../lib/categories";
import { cn } from "../../lib/cn";
import type { Target } from "../../types/generated/Target";
import { ApplyAllFooter, type ApplyAllProps } from "./ApplyAllFooter";

interface FormatPopoverProps {
  targets: Target[];
  selectedId: string | null;
  onSelect: (targetId: string) => void;
  /** Omitted for rows that can't fan out (merge rows, unknown formats). */
  applyAll?: Omit<ApplyAllProps, "target"> | undefined;
}

/** Matches label ("WebP image"), id ("webp", "anim-gif") and the format behind it. */
function matches(t: Target, q: string): boolean {
  if (!q) return true;
  const hay = `${t.label} ${t.id} ${t.kind.type === "convert" ? t.kind.format : ""}`.toLowerCase();
  return q.split(/\s+/).every((word) => hay.includes(word));
}

/**
 * Combobox, not a menu: focus stays in the search input the whole time and arrow keys
 * move a *virtual* highlight (aria-activedescendant). Moving real focus per option would
 * make every keystroke after an arrow press type into nowhere.
 */
export function FormatPopover({ targets, selectedId, onSelect, applyAll }: FormatPopoverProps) {
  const [query, setQuery] = useState("");
  const q = query.trim().toLowerCase();
  // Derived every render (≤ ~25 targets): cheaper than memo bookkeeping and never stale.
  const groups = groupTargets(targets.filter((t) => matches(t, q)));
  const flat = groups.flatMap((g) => g.targets);

  // Highlight starts on the current choice so Enter on open is a no-op confirm.
  const [active, setActive] = useState(() => Math.max(0, flat.findIndex((t) => t.id === selectedId)));
  // Clamp instead of syncing via effect: if filtering shrank the list, render-time math fixes it.
  const index = flat.length === 0 ? -1 : Math.min(active, flat.length - 1);
  const current = flat[index];

  const baseId = useId();
  const optionId = (i: number) => `${baseId}-opt-${i}`;

  // DOM sync is a legit effect: keep the highlighted option visible while arrowing.
  useEffect(() => {
    if (index >= 0) document.getElementById(`${baseId}-opt-${index}`)?.scrollIntoView({ block: "nearest" });
  }, [baseId, index]);

  const onKeyDown = (e: KeyboardEvent<HTMLInputElement>) => {
    const move = (to: number) => {
      e.preventDefault(); // stop the caret jumping to start/end of the input
      if (flat.length) setActive((to + flat.length) % flat.length);
    };
    switch (e.key) {
      case "ArrowDown":
        return move(index + 1);
      case "ArrowUp":
        return move(index - 1);
      case "Home":
        return e.ctrlKey ? move(0) : undefined;
      case "End":
        return e.ctrlKey ? move(flat.length - 1) : undefined;
      case "Enter":
        if (!current) return;
        e.preventDefault();
        // Keep app-level Ctrl+Enter (convert all) from firing while the picker owns the keyboard.
        e.stopPropagation();
        if (e.altKey && applyAll) applyAll.onApply(current.id, e.shiftKey);
        else onSelect(current.id);
    }
  };

  return (
    <div className="flex max-h-[inherit] w-72 flex-col overflow-hidden rounded-lg border border-line bg-surface shadow-2xl shadow-black/60">
      <label className="flex items-center gap-2 border-b border-line px-3 py-2">
        <Search size={14} className="shrink-0 text-muted" />
        <input
          autoFocus
          role="combobox"
          aria-expanded
          aria-controls={`${baseId}-list`}
          aria-activedescendant={index >= 0 ? optionId(index) : undefined}
          value={query}
          onChange={(e) => {
            setQuery(e.target.value);
            setActive(0); // new filter ⇒ best match first (event handler, not an effect)
          }}
          onKeyDown={onKeyDown}
          placeholder="Search formats…"
          spellCheck={false}
          className="min-w-0 flex-1 bg-transparent text-sm text-ink outline-none placeholder:text-muted/70"
        />
      </label>

      <div id={`${baseId}-list`} role="listbox" className="min-h-0 flex-1 overflow-y-auto p-1">
        {flat.length === 0 && <p className="px-2 py-6 text-center text-xs text-muted">No format matches “{query}”</p>}
        {groups.map(({ group, targets: items }) => (
          <section key={group} className="py-1" role="group" aria-label={CATEGORY_LABEL[group]}>
            <h3 className="flex items-center gap-1.5 px-2 pt-1 pb-1.5 text-[10px] font-semibold tracking-wider text-muted uppercase">
              <span className="size-1.5 rounded-full" style={{ backgroundColor: CATEGORY_TINT[group] }} />
              {CATEGORY_LABEL[group]}
            </h3>
            {items.map((t) => {
              const i = flat.indexOf(t);
              const chosen = t.id === selectedId;
              return (
                <div
                  key={t.id}
                  id={optionId(i)}
                  role="option"
                  aria-selected={chosen}
                  // mousedown + preventDefault keeps focus in the input (no blur flicker).
                  onMouseDown={(e) => e.preventDefault()}
                  onMouseMove={() => i !== index && setActive(i)}
                  onClick={() => onSelect(t.id)}
                  className={cn(
                    "flex cursor-default items-center gap-2 rounded-md px-2 py-1.5 text-sm",
                    i === index ? "bg-raised text-ink" : "text-ink/90",
                    chosen && "text-ink",
                  )}
                >
                  <span className="min-w-0 flex-1">
                    <span className="block truncate">{t.label}</span>
                    {t.note && <span className="block truncate text-[11px] text-muted">{t.note}</span>}
                  </span>
                  {chosen && <Check size={14} className="shrink-0 text-accent" />}
                </div>
              );
            })}
          </section>
        ))}
      </div>

      {applyAll && <ApplyAllFooter {...applyAll} target={current} />}
    </div>
  );
}

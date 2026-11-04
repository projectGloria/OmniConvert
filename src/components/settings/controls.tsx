import type { ReactNode } from "react";
import { cn } from "../../lib/cn";

/** Label + control row. `hint` sits right-aligned (current value, encoder chip…). */
export function Field({ label, hint, children }: { label: string; hint?: ReactNode; children: ReactNode }) {
  return (
    <div className="space-y-1.5">
      <div className="flex items-center justify-between gap-2 text-[11px]">
        <span className="font-medium tracking-wide text-muted uppercase">{label}</span>
        {hint}
      </div>
      {children}
    </div>
  );
}

export interface Option<V> {
  /** null = "unset / original" (no key sent to Rust). */
  value: V | null;
  label: string;
  disabled?: boolean;
  title?: string;
}

/**
 * Radio-group segmented control. Native radios underneath (visually hidden) give us
 * arrow-key roving focus and screen-reader semantics for free — re-implementing
 * roving tabindex by hand is exactly the kind of a11y code that rots.
 */
export function Segmented<V extends string | number>({
  name,
  options,
  value,
  onChange,
}: {
  name: string;
  options: readonly Option<V>[];
  value: V | null;
  onChange: (v: V | null) => void;
}) {
  return (
    <div role="radiogroup" className="flex rounded-md border border-line bg-base p-0.5">
      {options.map((o) => {
        const on = o.value === value;
        return (
          <label
            key={String(o.value)}
            title={o.title}
            className={cn(
              "flex-1 cursor-pointer rounded px-1.5 py-1 text-center text-xs whitespace-nowrap transition-colors",
              "has-[:focus-visible]:ring-1 has-[:focus-visible]:ring-accent",
              on ? "bg-raised text-ink shadow-[0_0_0_1px_var(--color-line)]" : "text-muted hover:text-ink",
              o.disabled && "pointer-events-none opacity-40",
            )}
          >
            <input
              type="radio"
              name={name}
              className="sr-only"
              checked={on}
              disabled={o.disabled}
              onChange={() => onChange(o.value)}
            />
            {o.label}
          </label>
        );
      })}
    </div>
  );
}

/** 1–100 slider. Commits on every input event — setSettings is a cheap single-row patch. */
export function Slider({ value, onChange, min = 1, max = 100 }: { value: number; onChange: (v: number) => void; min?: number; max?: number }) {
  return (
    <input
      type="range"
      min={min}
      max={max}
      value={value}
      onChange={(e) => onChange(e.target.valueAsNumber)}
      className="w-full accent-accent"
    />
  );
}

/** Free-form positive integer; empty = unset. Invalid input is ignored, not coerced to 0. */
export function NumberInput({ value, onChange, placeholder, suffix }: { value: number | null; onChange: (v: number | null) => void; placeholder: string; suffix: string }) {
  return (
    <label className="flex items-center gap-1 rounded-md border border-line bg-base px-2 py-1 text-xs focus-within:border-accent/60">
      <input
        type="number"
        min={1}
        max={16384}
        // Uncontrolled-ish: keyed on the value so external resets (chips) re-seed the field.
        key={value ?? "unset"}
        defaultValue={value ?? ""}
        placeholder={placeholder}
        onBlur={(e) => {
          const n = e.target.valueAsNumber;
          onChange(Number.isFinite(n) && n >= 1 ? Math.min(16384, Math.round(n)) : null);
        }}
        onKeyDown={(e) => e.key === "Enter" && e.currentTarget.blur()}
        className="w-16 bg-transparent text-ink outline-none [appearance:textfield] placeholder:text-muted/60 [&::-webkit-inner-spin-button]:appearance-none"
      />
      <span className="text-muted">{suffix}</span>
    </label>
  );
}

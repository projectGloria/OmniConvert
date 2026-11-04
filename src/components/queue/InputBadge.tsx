import type { Category } from "../../types/generated/Category";
import { CATEGORY_LABEL, CATEGORY_TINT } from "../../lib/categories";

interface InputBadgeProps {
  ext: string;
  category: Category | null;
}

/** Source-format chip, tinted by domain. Unsupported files get a neutral, dimmed chip. */
export function InputBadge({ ext, category }: InputBadgeProps) {
  const tint = category ? CATEGORY_TINT[category] : undefined;

  return (
    <span
      title={category ? CATEGORY_LABEL[category] : "Unsupported format"}
      // color-mix keeps one tint value driving text, border and fill — no per-category class soup.
      style={
        tint
          ? {
              color: tint,
              borderColor: `color-mix(in srgb, ${tint} 28%, transparent)`,
              backgroundColor: `color-mix(in srgb, ${tint} 7%, transparent)`,
            }
          : undefined
      }
      className="w-14 shrink-0 truncate rounded-md border border-line bg-base px-1 py-1 text-center font-mono text-[11px] font-medium tracking-wide text-muted uppercase"
    >
      {ext || "—"}
    </span>
  );
}

/** Source-format chip. Phase 2 tints it by Category once the registry exists. */
export function InputBadge({ ext }: { ext: string }) {
  return (
    <span className="w-14 shrink-0 rounded-md border border-line bg-base py-1 text-center font-mono text-[11px] font-medium tracking-wide text-ink uppercase">
      {ext || "—"}
    </span>
  );
}

/** Minimal className joiner — falsy entries are dropped. No need for clsx's object syntax here. */
export function cn(...classes: Array<string | false | null | undefined>): string {
  return classes.filter(Boolean).join(" ");
}

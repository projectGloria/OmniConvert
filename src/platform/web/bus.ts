import type { Unlisten } from "../types";

// In-page stand-in for Tauri's event system. The worker pool emits the exact
// conversion://* payloads the Rust queue emits, so useConversionEvents doesn't care
// which host it runs on.

type Handler = (payload: unknown) => void;
const handlers = new Map<string, Set<Handler>>();

export function on<T>(event: string, handler: (payload: T) => void): Unlisten {
  const set = handlers.get(event) ?? new Set<Handler>();
  handlers.set(event, set);
  const h = handler as Handler;
  set.add(h);
  return () => set.delete(h);
}

export function emit(event: string, payload: unknown): void {
  // Copy first: a handler that unsubscribes mid-dispatch must not skip its neighbour.
  for (const h of [...(handlers.get(event) ?? [])]) h(payload);
}

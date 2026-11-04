import { Titlebar } from "./components/titlebar/Titlebar";
import { DropOverlay } from "./components/dropzone/DropOverlay";
import { EmptyState } from "./components/dropzone/EmptyState";
import { QueueList } from "./components/queue/QueueList";
import { BottomBar } from "./components/bottombar/BottomBar";
import { useConversionEvents } from "./hooks/useConversionEvents";
import { useFileDrop } from "./hooks/useFileDrop";
import { useQueue } from "./store/queueStore";

export function App() {
  useConversionEvents();
  const isDragging = useFileDrop();
  // Boolean selector: App re-renders only when the queue flips empty↔non-empty,
  // not on every add/remove.
  const hasItems = useQueue((s) => s.order.length > 0);

  return (
    <div className="flex h-full flex-col">
      <Titlebar />
      <main className="relative min-h-0 flex-1">{hasItems ? <QueueList /> : <EmptyState />}</main>
      <BottomBar />
      <DropOverlay visible={isDragging} />
    </div>
  );
}

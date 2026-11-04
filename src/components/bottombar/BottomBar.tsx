import { ConvertAllButton } from "./ConvertAllButton";
import { OutputFolderPicker } from "./OutputFolderPicker";

export function BottomBar() {
  return (
    <footer className="flex h-14 shrink-0 items-center justify-between gap-4 border-t border-line bg-surface px-4">
      <OutputFolderPicker />
      <ConvertAllButton />
    </footer>
  );
}

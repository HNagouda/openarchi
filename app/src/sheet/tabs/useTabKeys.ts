// Switching sheet tabs from the keyboard: Ctrl+Tab and Ctrl+Shift+Tab, or
// Ctrl+PageDown and Ctrl+PageUp as in spreadsheets, everywhere; and ⌘⌥→ and
// ⌘⌥← on a Mac, as in Safari. Each steps one tab and wraps at the ends.

import { useKeyDown } from "../../lib/keys";
import { isMac } from "../../lib/platform";

type Direction = 1 | -1;

function directionOf(event: KeyboardEvent): Direction | null {
  if (event.ctrlKey && !event.metaKey && event.key === "Tab") return event.shiftKey ? -1 : 1;
  if (event.ctrlKey && event.key === "PageDown") return 1;
  if (event.ctrlKey && event.key === "PageUp") return -1;
  if (isMac && event.metaKey && event.altKey && event.key === "ArrowRight") return 1;
  if (isMac && event.metaKey && event.altKey && event.key === "ArrowLeft") return -1;
  return null;
}

export function useTabKeys(count: number, active: number, onSelect: (index: number) => void, isEnabled: boolean) {
  useKeyDown((event) => {
    if (!isEnabled || count < 2) return;
    if (document.querySelector("[data-modal-open]")) return;
    const direction = directionOf(event);
    if (direction === null) return;
    event.preventDefault();
    onSelect((active + direction + count) % count);
  });
}

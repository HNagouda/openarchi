// Keyboard handling shared by every screen. Single keys ([, ], D, I, arrows, Delete)
// never fire while the user is typing in a field.

import { useEffect, useRef } from "react";
import { isMac } from "./platform";

// Arrow keys nudge a selected mark or block by a millimetre of paper, or a centimetre with Shift.
const NUDGE_MM = 1;
const NUDGE_LARGE_MM = 10;

export function isTyping(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false;
  if (target.isContentEditable) return true;
  if (target instanceof HTMLTextAreaElement || target instanceof HTMLSelectElement) return true;
  if (target instanceof HTMLInputElement) {
    // Checkboxes and ranges do not take text, so shortcuts still work on them.
    return !["checkbox", "radio", "range", "button"].includes(target.type);
  }
  return false;
}

/** ⌘ on macOS, Ctrl elsewhere. */
export function hasModifier(event: KeyboardEvent): boolean {
  return isMac ? event.metaKey : event.ctrlKey;
}

/** Listens to keydown on the window while mounted, always calling the latest handler. */
export function useKeyDown(handler: (event: KeyboardEvent) => void) {
  const latest = useRef(handler);
  latest.current = handler;

  useEffect(() => {
    const listener = (event: KeyboardEvent) => latest.current(event);
    window.addEventListener("keydown", listener);
    return () => window.removeEventListener("keydown", listener);
  }, []);
}

/** Single-key shortcuts: skipped while typing, with a modifier held, or inside a dialog. */
export function usePlainKeys(handler: (key: string, event: KeyboardEvent) => void, isEnabled = true) {
  useKeyDown((event) => {
    if (!isEnabled) return;
    if (isTyping(event.target)) return;
    if (event.metaKey || event.ctrlKey || event.altKey) return;
    if (document.querySelector("[data-modal-open]")) return;
    handler(event.key, event);
  });
}

/** The nudge an arrow key asks for, [dx, dy] in paper millimetres, or null for any other key. */
export function arrowStep(event: KeyboardEvent): [number, number] | null {
  const step = event.shiftKey ? NUDGE_LARGE_MM : NUDGE_MM;
  if (event.key === "ArrowLeft") return [-step, 0];
  if (event.key === "ArrowRight") return [step, 0];
  if (event.key === "ArrowUp") return [0, -step];
  if (event.key === "ArrowDown") return [0, step];
  return null;
}

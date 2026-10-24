// App commands: the same ids come from the macOS menu bar and from Ctrl
// shortcuts on Windows, so each screen handles one list of names.

import { createContext, useContext, useEffect, useRef } from "react";

export type CommandId =
  | "new"
  | "open"
  | "export"
  | "undo"
  | "redo"
  | "settings"
  | "zoom-in"
  | "zoom-out"
  | "zoom-fit"
  | "inspector"
  | "diagrams"
  | "home"
  | "tour"
  | "assistant"
  | "shortcuts"
  | "add-text";

/** Returns true when the screen handled the command. */
export type CommandHandler = (id: CommandId) => boolean;

export const CommandContext = createContext<{ current: CommandHandler | null }>({ current: null });

/** Lets the visible screen answer commands while it is mounted. */
export function useCommandHandler(handler: CommandHandler) {
  const slot = useContext(CommandContext);
  const latest = useRef(handler);
  latest.current = handler;

  // A screen opened over another, like the report editor over the sheet,
  // hands the commands back to the one below when it closes.
  useEffect(() => {
    const forward: CommandHandler = (id) => latest.current(id);
    const below = slot.current;
    slot.current = forward;
    return () => {
      if (slot.current === forward) slot.current = below;
    };
  }, [slot]);
}

/** Windows and Linux have no menu bar, so Ctrl shortcuts map to commands here. */
export function commandForKey(key: string, isShift: boolean): CommandId | null {
  const lower = key.toLowerCase();
  if (lower === "z") return isShift ? "redo" : "undo";
  if (lower === "y") return "redo";
  if (lower === "n") return "new";
  if (lower === "o") return "open";
  if (lower === "e") return "export";
  if (lower === ",") return "settings";
  if (lower === "=" || lower === "+") return "zoom-in";
  if (lower === "-") return "zoom-out";
  if (lower === "0") return "zoom-fit";
  if (lower === "/") return "shortcuts";
  if (lower === "h" && isShift) return "home";
  return null;
}

// Platform conventions: ⌘ on macOS, Ctrl elsewhere, and the file manager's name.

export const isMac = navigator.userAgent.includes("Mac");

export const fileManagerName = isMac ? "Finder" : "Explorer";

const MAC_SYMBOLS: Record<string, string> = {
  ctrl: "⌃",
  tab: "⇥",
  pageup: "PgUp",
  pagedown: "PgDn",
  mod: "⌘",
  shift: "⇧",
  alt: "⌥",
  enter: "↩",
  delete: "⌫",
  escape: "esc",
  // "+" itself separates the parts of a shortcut, so the key is written "plus".
  plus: "+",
};

const OTHER_NAMES: Record<string, string> = {
  ctrl: "Ctrl",
  tab: "Tab",
  pageup: "PgUp",
  pagedown: "PgDn",
  mod: "Ctrl",
  shift: "Shift",
  alt: "Alt",
  enter: "Enter",
  delete: "Delete",
  escape: "Esc",
  plus: "+",
};

/** "mod+shift+z" -> "⇧⌘Z" on macOS, "Ctrl+Shift+Z" on Windows. */
export function shortcutLabel(shortcut: string): string {
  const parts = shortcut.split("+").map((part) => part.toLowerCase());
  if (isMac) {
    // macOS lists modifiers in a fixed order: ⌃⌥⇧⌘.
    const order = ["ctrl", "alt", "shift", "mod"];
    const modifiers = order.filter((name) => parts.includes(name)).map((name) => MAC_SYMBOLS[name]);
    const keys = parts.filter((part) => !order.includes(part)).map((key) => MAC_SYMBOLS[key] ?? key.toUpperCase());
    return modifiers.join("") + keys.join("");
  }

  return parts.map((part) => OTHER_NAMES[part] ?? part.toUpperCase()).join("+");
}

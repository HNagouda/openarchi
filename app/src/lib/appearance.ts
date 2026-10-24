// System, light, dark or AMOLED black. Stored per machine, outside undo.
// index.html applies the stored choice before the first paint; this keeps it current.

import { useEffect, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";

// AMOLED is dark with true black surfaces, for OLED screens and dim rooms.
export type Appearance = "system" | "light" | "dark" | "amoled";

const STORAGE_KEY = "openarchi.appearance";
const APPEARANCES: Appearance[] = ["system", "light", "dark", "amoled"];

function readStored(): Appearance {
  try {
    const stored = localStorage.getItem(STORAGE_KEY);
    if (APPEARANCES.includes(stored as Appearance)) return stored as Appearance;
  } catch {
    // Storage can be unavailable; the system setting is a fine default.
  }
  return "system";
}

const darkQuery = window.matchMedia("(prefers-color-scheme: dark)");

/** The chosen appearance and whether the window currently shows dark. */
export function useAppearance() {
  const [appearance, setAppearance] = useState<Appearance>(readStored);
  const [isSystemDark, setSystemDark] = useState(darkQuery.matches);

  useEffect(() => {
    const listener = (event: MediaQueryListEvent) => setSystemDark(event.matches);
    darkQuery.addEventListener("change", listener);
    return () => darkQuery.removeEventListener("change", listener);
  }, []);

  const isBlack = appearance === "amoled";
  const isDark = isBlack || appearance === "dark" || (appearance === "system" && isSystemDark);

  useEffect(() => {
    const root = document.documentElement;
    // Black is a kind of dark: everything keyed on dark still applies, and the
    // black tokens override only the surfaces.
    root.dataset.appearance = isDark ? "dark" : "light";
    if (isBlack) root.dataset.black = "";
    else delete root.dataset.black;
    // Native parts (traffic lights, scroll bars, dialogs) follow the same choice.
    const nativeTheme = appearance === "system" ? null : isDark ? "dark" : "light";
    getCurrentWindow()
      .setTheme(nativeTheme)
      .catch(() => undefined);
  }, [isDark, isBlack, appearance]);

  const choose = (next: Appearance) => {
    setAppearance(next);
    try {
      localStorage.setItem(STORAGE_KEY, next);
    } catch {
      // Not remembered across launches, but applied now.
    }
  };

  return { appearance, isDark, choose };
}

/** Whether the window shows dark right now, for parts that pick their own assets (the basemap). */
export function useIsDark() {
  const read = () => document.documentElement.dataset.appearance === "dark";
  const [isDark, setDark] = useState(read);

  useEffect(() => {
    const observer = new MutationObserver(() => setDark(read()));
    observer.observe(document.documentElement, { attributes: true, attributeFilter: ["data-appearance"] });
    return () => observer.disconnect();
  }, []);

  return isDark;
}

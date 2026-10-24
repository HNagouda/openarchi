// Light, dark or follow the system. Stored per machine, outside undo.

import { useEffect, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";

export type Appearance = "system" | "light" | "dark";

const STORAGE_KEY = "openarchi.appearance";

function readStored(): Appearance {
  try {
    const stored = localStorage.getItem(STORAGE_KEY);
    if (stored === "light" || stored === "dark" || stored === "system") return stored;
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

  const isDark = appearance === "dark" || (appearance === "system" && isSystemDark);

  useEffect(() => {
    document.documentElement.dataset.appearance = isDark ? "dark" : "light";
    // Native parts (traffic lights, scroll bars, dialogs) follow the same choice.
    getCurrentWindow()
      .setTheme(appearance === "system" ? null : appearance)
      .catch(() => undefined);
  }, [isDark, appearance]);

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

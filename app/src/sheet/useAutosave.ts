// Saves the look a moment after the last change. The library keeps a backup
// of every earlier version, so saving often is safe.

import { useCallback, useEffect, useRef, useState } from "react";
import { errorText, saveLook } from "../lib/ipc";
import type { Look } from "../lib/types";

// Long enough that a slider drag saves once, at the end.
const SAVE_DELAY_MS = 700;

export type SaveStatus = "saved" | "pending" | "saving" | "failed";

export function useAutosave(look: Look, onError: (message: string) => void) {
  const [status, setStatus] = useState<SaveStatus>("saved");
  const savedKey = useRef(JSON.stringify(look));
  const latestLook = useRef(look);
  const timer = useRef(0);
  latestLook.current = look;

  const save = useCallback(async () => {
    window.clearTimeout(timer.current);
    const target = latestLook.current;
    const key = JSON.stringify(target);
    if (key === savedKey.current) {
      setStatus("saved");
      return;
    }
    setStatus("saving");
    try {
      await saveLook(target);
      savedKey.current = key;
      setStatus(JSON.stringify(latestLook.current) === key ? "saved" : "pending");
    } catch (error) {
      setStatus("failed");
      onError(errorText(error));
    }
  }, [onError]);

  const key = JSON.stringify(look);
  useEffect(() => {
    if (key === savedKey.current) return;
    setStatus("pending");
    window.clearTimeout(timer.current);
    timer.current = window.setTimeout(save, SAVE_DELAY_MS);
  }, [key, save]);

  /** Marks a look read from disk as saved, so it is not written straight back. */
  const markSaved = useCallback((saved: Look) => {
    savedKey.current = JSON.stringify(saved);
    setStatus("saved");
  }, []);

  return { status, flush: save, markSaved, hasUnsaved: status === "pending" || status === "saving" };
}

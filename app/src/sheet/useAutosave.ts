// Saves the look, or a project's reports, a moment after the last change. The
// library keeps a backup of every earlier version, so saving often is safe.

import { useCallback, useEffect, useRef, useState } from "react";
import { errorText, saveLook } from "../lib/ipc";
import type { Look } from "../lib/types";

// Long enough that a slider drag saves once, at the end.
const SAVE_DELAY_MS = 700;

export type SaveStatus = "saved" | "pending" | "saving" | "failed";

export function useAutosave(look: Look, onError: (message: string) => void) {
  return useSaving(look, saveLook, onError);
}

/** Writes `value` with `write` a moment after it last changed. */
export function useSaving<T>(value: T, write: (value: T) => Promise<unknown>, onError: (message: string) => void) {
  const [status, setStatus] = useState<SaveStatus>("saved");
  const savedKey = useRef(JSON.stringify(value));
  const latestValue = useRef(value);
  const timer = useRef(0);
  latestValue.current = value;

  const save = useCallback(async () => {
    window.clearTimeout(timer.current);
    const target = latestValue.current;
    const key = JSON.stringify(target);
    if (key === savedKey.current) {
      setStatus("saved");
      return;
    }
    setStatus("saving");
    try {
      await write(target);
      savedKey.current = key;
      setStatus(JSON.stringify(latestValue.current) === key ? "saved" : "pending");
    } catch (error) {
      setStatus("failed");
      onError(errorText(error));
    }
  }, [onError, write]);

  const key = JSON.stringify(value);
  useEffect(() => {
    if (key === savedKey.current) return;
    setStatus("pending");
    window.clearTimeout(timer.current);
    timer.current = window.setTimeout(save, SAVE_DELAY_MS);
  }, [key, save]);

  /** Marks a value read from disk as saved, so it is not written straight back. */
  const markSaved = useCallback((saved: T) => {
    savedKey.current = JSON.stringify(saved);
    setStatus("saved");
  }, []);

  return { status, flush: save, markSaved, hasUnsaved: status === "pending" || status === "saving" };
}

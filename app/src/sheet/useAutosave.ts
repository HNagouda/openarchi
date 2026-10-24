// Saves a moment after the last change. The library keeps a backup of every
// earlier version, so saving often is safe.

import { useCallback, useEffect, useRef, useState } from "react";
import { errorText } from "../lib/ipc";

// Long enough that a slider drag saves once, at the end.
const SAVE_DELAY_MS = 700;

export type SaveStatus = "saved" | "pending" | "saving" | "failed";

/** Writes `value` with `write` once it settles; `write` is read when the save runs, so it may change. */
export function useAutosave<T>(value: T, write: (value: T) => Promise<unknown>, onError: (message: string) => void) {
  const [status, setStatus] = useState<SaveStatus>("saved");
  const savedKey = useRef(JSON.stringify(value));
  const latest = useRef({ value, write });
  const timer = useRef(0);
  latest.current = { value, write };

  const save = useCallback(async () => {
    window.clearTimeout(timer.current);
    const target = latest.current.value;
    const key = JSON.stringify(target);
    if (key === savedKey.current) {
      setStatus("saved");
      return;
    }
    setStatus("saving");
    try {
      await latest.current.write(target);
      savedKey.current = key;
      setStatus(JSON.stringify(latest.current.value) === key ? "saved" : "pending");
    } catch (error) {
      setStatus("failed");
      onError(errorText(error));
    }
  }, [onError]);

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

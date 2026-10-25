// Saves a moment after the last change. The library keeps a backup of every
// earlier version, so saving often is safe.

import { useCallback, useEffect, useRef, useState } from "react";
import { errorText } from "../lib/ipc";

// Long enough that a slider drag saves once, at the end.
const SAVE_DELAY_MS = 700;

type SaveStatus = "saved" | "pending" | "saving" | "failed";

/** Writes `value` with `write` once it settles; `write` is read when the save runs, so it may change.
 * `value` comes from the undo history, which keeps a snapshot until it changes, so a new object means a change. */
export function useAutosave<T>(value: T, write: (value: T) => Promise<unknown>, onError: (message: string) => void) {
  const [status, setStatus] = useState<SaveStatus>("saved");
  // The value last written, or read from disk.
  const saved = useRef(value);
  const latest = useRef({ value, write });
  const timer = useRef(0);
  latest.current = { value, write };

  const save = useCallback(async () => {
    window.clearTimeout(timer.current);
    const target = latest.current.value;
    // Compared by content here, once per save, so an edit that was undone again writes nothing.
    if (JSON.stringify(target) === JSON.stringify(saved.current)) {
      saved.current = target;
      setStatus("saved");
      return;
    }
    setStatus("saving");
    try {
      await latest.current.write(target);
      saved.current = target;
      setStatus(latest.current.value === target ? "saved" : "pending");
    } catch (error) {
      setStatus("failed");
      onError(errorText(error));
    }
  }, [onError]);

  useEffect(() => {
    if (value === saved.current) return;
    setStatus("pending");
    window.clearTimeout(timer.current);
    timer.current = window.setTimeout(save, SAVE_DELAY_MS);
  }, [value, save]);

  /** Marks a value read from disk as saved, so it is not written straight back. */
  const markSaved = useCallback((value: T) => {
    saved.current = value;
    setStatus("saved");
  }, []);

  return { status, flush: save, markSaved, hasUnsaved: status === "pending" || status === "saving" };
}

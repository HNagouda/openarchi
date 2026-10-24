// Undo for the look: a stack of whole `Look` snapshots.
//
// A slider drag sends many changes with the same merge key; they replace the
// newest snapshot instead of stacking, so the drag undoes in one step. `seal`
// ends the merge when the pointer lifts or a field loses focus.

import { useCallback, useRef, useState } from "react";
import type { Look } from "./types";

// Plenty for a session; old steps fall off the bottom.
const MAX_STEPS = 200;

type History = { past: Look[]; present: Look; future: Look[] };

export function useLookHistory(initial: Look) {
  const [history, setHistory] = useState<History>({ past: [], present: initial, future: [] });
  const mergeKey = useRef<string | null>(null);

  const change = useCallback((next: Look, key?: string) => {
    setHistory((current) => {
      if (JSON.stringify(next) === JSON.stringify(current.present)) return current;
      const isMerge = key !== undefined && key === mergeKey.current;
      mergeKey.current = key ?? null;
      if (isMerge) return { ...current, present: next };
      const past = [...current.past, current.present].slice(-MAX_STEPS);
      return { past, present: next, future: [] };
    });
  }, []);

  const seal = useCallback(() => {
    mergeKey.current = null;
  }, []);

  const undo = useCallback(() => {
    mergeKey.current = null;
    setHistory((current) => {
      const previous = current.past[current.past.length - 1];
      if (!previous) return current;
      return { past: current.past.slice(0, -1), present: previous, future: [current.present, ...current.future] };
    });
  }, []);

  const redo = useCallback(() => {
    mergeKey.current = null;
    setHistory((current) => {
      const next = current.future[0];
      if (!next) return current;
      return { past: [...current.past, current.present], present: next, future: current.future.slice(1) };
    });
  }, []);

  /** Replaces the look from outside (an assistant's edit) as one undoable step. */
  const replace = useCallback((next: Look) => {
    mergeKey.current = null;
    setHistory((current) => ({ past: [...current.past, current.present].slice(-MAX_STEPS), present: next, future: [] }));
  }, []);

  return {
    look: history.present,
    canUndo: history.past.length > 0,
    canRedo: history.future.length > 0,
    change,
    seal,
    undo,
    redo,
    replace,
  };
}

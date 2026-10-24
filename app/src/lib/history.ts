// Undo as a stack of whole snapshots: of the look, or of a project's reports.
//
// A slider drag sends many changes with the same merge key; they replace the
// newest snapshot instead of stacking, so the drag undoes in one step. `seal`
// ends the merge when the pointer lifts or a field loses focus.

import { useCallback, useRef, useState } from "react";
import type { Look } from "./types";

// Plenty for a session; old steps fall off the bottom.
const MAX_STEPS = 200;

type Stack<T> = { past: T[]; present: T; future: T[] };

export function useHistory<T>(initial: T) {
  const [history, setHistory] = useState<Stack<T>>({ past: [], present: initial, future: [] });
  const mergeKey = useRef<string | null>(null);

  const change = useCallback((next: T, key?: string) => {
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
      if (previous === undefined) return current;
      return { past: current.past.slice(0, -1), present: previous, future: [current.present, ...current.future] };
    });
  }, []);

  const redo = useCallback(() => {
    mergeKey.current = null;
    setHistory((current) => {
      if (current.future.length === 0) return current;
      const [next, ...rest] = current.future;
      return { past: [...current.past, current.present], present: next, future: rest };
    });
  }, []);

  /** Replaces the value from outside (an assistant's edit) as one undoable step. */
  const replace = useCallback((next: T) => {
    mergeKey.current = null;
    setHistory((current) => ({ past: [...current.past, current.present].slice(-MAX_STEPS), present: next, future: [] }));
  }, []);

  return {
    value: history.present,
    canUndo: history.past.length > 0,
    canRedo: history.future.length > 0,
    change,
    seal,
    undo,
    redo,
    replace,
  };
}

export function useLookHistory(initial: Look) {
  const history = useHistory(initial);
  return { ...history, look: history.value };
}

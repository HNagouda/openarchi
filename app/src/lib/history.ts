// Undo as a stack of whole snapshots, used for every undo in the app: the
// drawn site, a project's sheets and pins, its reports and a palette draft.
//
// A slider drag sends many changes with the same merge key; they replace the
// newest snapshot instead of stacking, so the drag undoes in one step. `seal`
// ends the merge when the pointer lifts or a field loses focus.

import { useCallback, useRef, useState } from "react";

// Plenty for a session; old steps fall off the bottom.
const MAX_STEPS = 200;

type History<T> = { past: T[]; present: T; future: T[] };

export function useHistory<T>(initial: T) {
  const [history, setHistory] = useState<History<T>>({ past: [], present: initial, future: [] });
  const mergeKey = useRef<string | null>(null);

  const change = useCallback((next: T, key?: string) => {
    // Decided here, not inside the updater: StrictMode runs updaters twice, and
    // batched map events can run one after the pointer lifted and `seal` ran.
    const isMerge = key !== undefined && key === mergeKey.current;
    mergeKey.current = key ?? null;
    setHistory((current) => {
      if (JSON.stringify(next) === JSON.stringify(current.present)) return current;
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
      if (current.past.length === 0) return current;
      const previous = current.past[current.past.length - 1];
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

  /** Replaces the snapshot from outside (an assistant's edit) as one undoable step. */
  const replace = useCallback((next: T) => {
    mergeKey.current = null;
    setHistory((current) => ({ past: [...current.past, current.present].slice(-MAX_STEPS), present: next, future: [] }));
  }, []);

  return {
    present: history.present,
    canUndo: history.past.length > 0,
    canRedo: history.future.length > 0,
    change,
    seal,
    undo,
    redo,
    replace,
  };
}

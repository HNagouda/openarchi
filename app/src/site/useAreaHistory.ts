// Undo for the site step: a stack of drawn areas, with null for "nothing drawn".
//
// The same shape as the sheet's look history (lib/history.ts): a drag sends
// many changes with one merge key, which replace the newest step instead of
// stacking, and `seal` ends the merge when the mouse lifts. The history lives
// and dies with the site step, so leaving it starts the next visit fresh.

import { useCallback, useRef, useState } from "react";
import type { Area } from "../lib/types";

// Plenty for one drawing session; old steps fall off the bottom.
const MAX_STEPS = 200;

type Present = Area | null;
type History = { past: Present[]; present: Present; future: Present[] };

export function useAreaHistory(initial: Present) {
  const [history, setHistory] = useState<History>({ past: [], present: initial, future: [] });
  const mergeKey = useRef<string | null>(null);

  const change = useCallback((next: Present, key?: string) => {
    // Decided now, not inside the updater: map events are batched, so an
    // updater can run after the mouse lifted and `seal` already ran.
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
      return { past: [...current.past, current.present], present: current.future[0], future: current.future.slice(1) };
    });
  }, []);

  return {
    area: history.present,
    canUndo: history.past.length > 0,
    canRedo: history.future.length > 0,
    change,
    seal,
    undo,
    redo,
  };
}

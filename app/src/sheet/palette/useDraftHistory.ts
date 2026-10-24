// Undo inside the palette editor: a stack of whole drafts, the way
// lib/history.ts keeps looks. A drag in the colour picker sends many changes
// with one merge key, which replace the newest step instead of stacking.

import { useCallback, useRef, useState } from "react";
import type { PaletteDraft } from "../../lib/palette";

// More than anyone undoes inside one dialog.
const MAX_STEPS = 100;

type Stack = { past: PaletteDraft[]; present: PaletteDraft; future: PaletteDraft[] };

export function useDraftHistory(initial: PaletteDraft) {
  const [stack, setStack] = useState<Stack>({ past: [], present: initial, future: [] });
  const mergeKey = useRef<string | null>(null);

  // The merge is decided outside the state updater, which StrictMode runs twice.
  const change = useCallback((next: PaletteDraft, key?: string) => {
    const isMerge = key !== undefined && key === mergeKey.current;
    mergeKey.current = key ?? null;
    setStack((current) => {
      if (isMerge) return { ...current, present: next };
      return { past: [...current.past, current.present].slice(-MAX_STEPS), present: next, future: [] };
    });
  }, []);

  const undo = useCallback(() => {
    mergeKey.current = null;
    setStack((current) => {
      const previous = current.past[current.past.length - 1];
      if (!previous) return current;
      return { past: current.past.slice(0, -1), present: previous, future: [current.present, ...current.future] };
    });
  }, []);

  const redo = useCallback(() => {
    mergeKey.current = null;
    setStack((current) => {
      const next = current.future[0];
      if (!next) return current;
      return { past: [...current.past, current.present], present: next, future: current.future.slice(1) };
    });
  }, []);

  // Ends a merge when the pointer lifts, so two drags of one swatch undo apart.
  const seal = useCallback(() => {
    mergeKey.current = null;
  }, []);

  return { draft: stack.present, canUndo: stack.past.length > 0, canRedo: stack.future.length > 0, change, seal, undo, redo };
}

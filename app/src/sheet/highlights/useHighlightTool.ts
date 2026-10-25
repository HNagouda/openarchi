// Drawing, selecting, reshaping, moving and deleting highlights on the sheet.
//
// Drawing a new one is in useHighlightDrawing. A selected highlight drags by
// its body or its handles. Like pins, only the drop changes the highlights, so
// every edit is one undo step, and paper points go through the sheet's ground
// points into RD New metres, so a shape drawn on a tilted view lies on the
// same ground in every other view. An explode view draws highlights on every
// plate but carries the top plate's ground, so they are edited there, as pins are.

import { useEffect, useRef, useState, type RefObject } from "react";
import { arrowStep, usePlainKeys } from "../../lib/keys";
import type { Highlight, HighlightShape, Look } from "../../lib/types";
import type { CanvasControls, PaperView } from "../Canvas";
import { useLiveValue } from "../liveValue";
import { paperToRd, rdToPaper, type Point } from "../paper";
import { handlesOf, moved, scaled, withChangedHighlight, withHandleMoved, withoutHighlight } from "./highlightEdits";
import { followOnGround, isNear, paperOf, problemWith } from "./highlightGround";
import { highlightAt } from "./sheetHighlights";
import { useHighlightDrawing } from "./useHighlightDrawing";

export type { HighlightDraft } from "./useHighlightDrawing";

export const HIGHLIGHT_KEY = "h";
// + and - grow and shrink a selected highlight by a tenth.
const SCALE_STEP = 1.1;
const SCALE_KEYS: Record<string, number> = { "+": SCALE_STEP, "=": SCALE_STEP, "-": 1 / SCALE_STEP };

/** A highlight's shape while it is moved or reshaped, until the core draws it. */
export type HighlightPreview = { id: string; shape: HighlightShape };

type Options = {
  highlights: Highlight[];
  onHighlights: (next: Highlight[], mergeKey?: string) => void;
  canvas: RefObject<CanvasControls | null>;
  look: Look;
  svg: string | null;
  isEnabled: boolean;
  onRefused: (message: string) => void;
};

export function useHighlightTool({ highlights, onHighlights, canvas, look, svg, isEnabled, onRefused }: Options) {
  const preview = useLiveValue<HighlightPreview | null>(null);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [editingId, setEditingId] = useState<string | null>(null);
  const latest = useRef(highlights);
  latest.current = highlights;
  const isAvailable = look.view.mode !== "section";

  // A new highlight is selected with its label open for typing.
  const selectNew = (id: string) => {
    setSelectedId(id);
    setEditingId(id);
  };
  const letGo = () => {
    setSelectedId(null);
    setEditingId(null);
  };
  const drawing = useHighlightDrawing({
    latest,
    onHighlights,
    canvas,
    isAvailable,
    isEnabled,
    onAdded: selectNew,
    onStart: () => setSelectedId(null),
    onRefused,
  });

  // The core's new SVG has the shape where the drop put it.
  useEffect(() => preview.set(null), [preview, svg]);
  // A highlight that undo took away is no longer selected.
  const isPresent = selectedId === null || highlights.some((highlight) => highlight.id === selectedId);
  useEffect(() => {
    if (!isPresent) letGo();
  }, [isPresent]);

  const changeShape = (id: string, shape: HighlightShape, paper: Element | null) => {
    const problem = problemWith(shape, paper);
    if (problem) {
      onRefused(problem);
      preview.set(null);
      return;
    }
    onHighlights(withChangedHighlight(latest.current, id, { shape }));
  };

  /** Picks a highlight, or one of the selected one's handles, and drags it. */
  const pressToEdit = (event: React.PointerEvent, view: PaperView): boolean => {
    const paper = view.getPaper();
    const pressed = view.toMm(event.clientX, event.clientY);
    const selected = latest.current.find((highlight) => highlight.id === selectedId);
    const toPaper = rdToPaper(paper);
    const handle = selected && toPaper ? handlesOf(selected.shape).findIndex((point) => isNear(toPaper(point), pressed)) : -1;
    if (selected && handle >= 0) {
      event.preventDefault();
      followOnGround(
        event,
        view,
        (at) => preview.set({ id: selected.id, shape: withHandleMoved(selected.shape, handle, at) }),
        (at) => at && changeShape(selected.id, withHandleMoved(selected.shape, handle, at), paper),
      );
      return true;
    }

    const id = highlightAt(paper, pressed);
    const hit = latest.current.find((highlight) => highlight.id === id);
    if (!hit) {
      letGo();
      return false;
    }
    event.preventDefault();
    setSelectedId(hit.id);
    setEditingId(null);
    const start = paperToRd(paper)?.(pressed);
    if (!start) return true;
    followOnGround(
      event,
      view,
      (at) => preview.set({ id: hit.id, shape: moved(hit.shape, at.x - start.x, at.y - start.y) }),
      (at) => at && changeShape(hit.id, moved(hit.shape, at.x - start.x, at.y - start.y), paper),
    );
    return true;
  };

  const onPaperPointerDown = (event: React.PointerEvent, view: PaperView): boolean => {
    if (drawing.isDrawing) {
      event.preventDefault();
      drawing.press(event, view);
      return true;
    }
    if (!isAvailable) return false;
    return pressToEdit(event, view);
  };

  const onPaperDoubleClick = (): boolean => {
    if (!drawing.isDrawing || drawing.kind !== "polygon") return false;
    drawing.finishPolygon();
    return true;
  };

  const remove = (id: string) => {
    onHighlights(withoutHighlight(latest.current, id));
    letGo();
  };

  const changeSelected = (change: (shape: HighlightShape, toRd: (point: Point) => Point) => HighlightShape) => {
    const paper = paperOf();
    const toRd = paperToRd(paper);
    const selected = latest.current.find((highlight) => highlight.id === selectedId);
    if (selected && toRd) changeShape(selected.id, change(selected.shape, toRd), paper);
  };

  usePlainKeys((key, event) => {
    if (key.toLowerCase() === HIGHLIGHT_KEY && !event.repeat) {
      if (drawing.isDrawing) drawing.stopDrawing();
      else drawing.startDrawing(drawing.kind);
      return;
    }
    if (drawing.isDrawing) {
      if (key === "Escape") drawing.stopDrawing();
      if (key === "Enter" && drawing.kind === "polygon") drawing.finishPolygon();
      if (key === "Backspace" && drawing.kind === "polygon") drawing.dropLastCorner();
      return;
    }
    if (!selectedId) return;
    if (key === "Escape") setSelectedId(null);
    if (key === "Enter") {
      event.preventDefault();
      setEditingId(selectedId);
    }
    if ((key === "Delete" || key === "Backspace") && !event.repeat) remove(selectedId);
    const scale = SCALE_KEYS[key];
    if (scale) {
      event.preventDefault();
      changeSelected((shape) => scaled(shape, scale));
    }
    const move = arrowStep(event);
    if (move) {
      event.preventDefault();
      changeSelected((shape, toRd) => {
        const [from, to] = [toRd({ x: 0, y: 0 }), toRd({ x: move[0], y: move[1] })];
        return moved(shape, to.x - from.x, to.y - from.y);
      });
    }
  }, isEnabled && editingId === null);

  return {
    kind: drawing.kind,
    isDrawing: drawing.isDrawing,
    isAvailable,
    draft: drawing.draft,
    preview,
    selectedId,
    editingId,
    setSelectedId,
    setEditingId,
    onPaperPointerDown,
    onPaperDoubleClick,
    startDrawing: drawing.startDrawing,
    stopDrawing: drawing.stopDrawing,
    placeInView: drawing.placeInView,
    remove,
  };
}

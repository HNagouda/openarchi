// Drawing, selecting, reshaping, moving and deleting highlights on the sheet.
//
// With the tool on, a rectangle is dragged from corner to corner and a circle
// from its centre outwards; a click instead places one of a house's size
// there. A polygon takes a click per corner and closes on a double-click,
// Return or a click on its first corner. A selected highlight drags by its
// body or its handles. Like pins, only the drop changes the highlights, so
// every edit is one undo step, and paper points go through the sheet's ground
// points into RD New metres, so a shape drawn on a tilted view lies on the
// same ground in every other view. An explode view draws highlights on its
// bottom plate, whose ground the sheet does not carry, so it is not edited there.

import { useEffect, useRef, useState, type RefObject } from "react";
import { usePlainKeys } from "../../lib/keys";
import type { Highlight, HighlightKind, HighlightShape, Look } from "../../lib/types";
import type { CanvasControls, PaperView } from "../Canvas";
import { paperToRd, type Point } from "../pins/sheetPins";
import {
  areaOf,
  handlesOf,
  middleOf,
  MIN_AREA_M2,
  moved,
  newHighlight,
  nextHighlightId,
  placedShape,
  ringOf,
  scaled,
  shapeFrom,
  withChangedHighlight,
  withHandleMoved,
  withHighlight,
  withoutHighlight,
} from "./highlightEdits";
import { highlightAt, rdToPaper, svgPoint } from "./sheetHighlights";

export const HIGHLIGHT_KEY = "h";
// A press that moves less than this is a click, not a drag.
const DRAG_THRESHOLD_PX = 4;
// How close to a handle or a polygon's first corner, in paper millimetres, a press still takes it.
const HANDLE_HIT_MM = 2.5;
// Arrow keys nudge by a millimetre of paper, or a centimetre with Shift; + and - grow and shrink by a tenth.
const NUDGE_MM = 1;
const NUDGE_LARGE_MM = 10;
const SCALE_STEP = 1.1;
const SCALE_KEYS: Record<string, number> = { "+": SCALE_STEP, "=": SCALE_STEP, "-": 1 / SCALE_STEP };
// The outline of the area the sheet draws, which the core writes in plan and axonometric views.
const BOUNDARY_SELECTOR = "#site-boundary";

/** What the overlay draws while the pointer works: RD New points, closed or still being drawn. */
export type HighlightDraft = { ring: Point[]; isOpen: boolean };

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
  const [kind, setKind] = useState<HighlightKind>("rectangle");
  const [isDrawing, setDrawing] = useState(false);
  const [corners, setCorners] = useState<Point[]>([]);
  const [draft, setDraft] = useState<HighlightDraft | null>(null);
  const [preview, setPreview] = useState<{ id: string; shape: HighlightShape } | null>(null);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [editingId, setEditingId] = useState<string | null>(null);
  // While a polygon is drawn, how to stop its next side following the pointer.
  const stopFollowing = useRef<(() => void) | null>(null);
  const latest = useRef(highlights);
  latest.current = highlights;
  const latestCorners = useRef(corners);
  latestCorners.current = corners;
  const isAvailable = look.view.mode === "plan" || look.view.mode === "axonometric";
  const unavailable = "Highlights are drawn and edited on a plan or an axonometric view.";

  // The core's new SVG has the shape where the drop put it.
  useEffect(() => setPreview(null), [svg]);
  // A highlight that undo took away is no longer selected.
  const isPresent = selectedId === null || highlights.some((highlight) => highlight.id === selectedId);
  useEffect(() => {
    if (!isPresent) {
      setSelectedId(null);
      setEditingId(null);
    }
  }, [isPresent]);
  useEffect(() => {
    if (!isAvailable || !isEnabled) stopDrawing();
  }, [isAvailable, isEnabled]);

  function stopDrawing() {
    stopFollowing.current?.();
    stopFollowing.current = null;
    setDrawing(false);
    setCorners([]);
    setDraft(null);
  }

  const paperOf = () => document.querySelector(".paper");

  /** Why a shape cannot be kept, or null when it can. */
  const problemWith = (shape: HighlightShape, paper: Element | null): string | null => {
    if (areaOf(shape) < MIN_AREA_M2) return `A highlight needs at least ${MIN_AREA_M2} m². Draw it a little larger.`;
    const toPaper = rdToPaper(paper);
    const boundary = paper?.querySelector<SVGPathElement>(BOUNDARY_SELECTOR);
    const middle = toPaper?.(middleOf(shape));
    if (middle && boundary && !boundary.isPointInFill(svgPoint(boundary, middle))) {
      return "The middle of a highlight goes inside the outline of the sheet's area.";
    }
    return null;
  };

  const addShape = (shape: HighlightShape, paper: Element | null) => {
    const problem = problemWith(shape, paper);
    if (problem) {
      onRefused(problem);
      setDraft(null);
      return;
    }
    const id = nextHighlightId(latest.current);
    // Drawing and typing the first label merge into one undo step.
    onHighlights(withHighlight(latest.current, newHighlight(id, shape)), `highlight:${id}`);
    stopDrawing();
    setSelectedId(id);
    setEditingId(id);
  };

  const changeShape = (id: string, shape: HighlightShape, paper: Element | null) => {
    const problem = problemWith(shape, paper);
    if (problem) {
      onRefused(problem);
      setPreview(null);
      return;
    }
    onHighlights(withChangedHighlight(latest.current, id, { shape }));
  };

  /** Follows the pointer from a press until it is let go, telling `onDrag` where it is in RD New. */
  const follow = (event: React.PointerEvent, view: PaperView, onDrag: (at: Point) => void, onDrop: (at: Point | null) => void) => {
    const toRd = paperToRd(view.getPaper());
    if (!toRd) return;
    const pressedAt = { x: event.clientX, y: event.clientY };
    let isDragging = false;
    const at = (pointer: PointerEvent) => toRd(view.toMm(pointer.clientX, pointer.clientY));
    const onMove = (move: PointerEvent) => {
      if (!isDragging && Math.hypot(move.clientX - pressedAt.x, move.clientY - pressedAt.y) < DRAG_THRESHOLD_PX) return;
      isDragging = true;
      onDrag(at(move));
    };
    const onUp = (up: PointerEvent) => {
      window.removeEventListener("pointermove", onMove);
      window.removeEventListener("pointerup", onUp);
      onDrop(isDragging ? at(up) : null);
    };
    window.addEventListener("pointermove", onMove);
    window.addEventListener("pointerup", onUp);
  };

  const pressWhileDrawing = (event: React.PointerEvent, view: PaperView) => {
    const paper = view.getPaper();
    const toRd = paperToRd(paper);
    if (!toRd) {
      onRefused(unavailable);
      return;
    }
    const start = toRd(view.toMm(event.clientX, event.clientY));
    if (kind === "polygon") {
      const toPaper = rdToPaper(paper);
      const first = corners[0] && toPaper?.(corners[0]);
      const pressed = view.toMm(event.clientX, event.clientY);
      const isOnFirst = first && Math.hypot(pressed.x - first.x, pressed.y - first.y) <= HANDLE_HIT_MM;
      if (isOnFirst && corners.length >= 3) {
        addShape(shapeFrom("polygon", corners), paper);
        return;
      }
      const next = [...corners, start];
      setCorners(next);
      setDraft({ ring: next, isOpen: true });
      // The next side follows the pointer until the next click.
      stopFollowing.current?.();
      const onMove = (move: PointerEvent) => {
        const at = toRd(view.toMm(move.clientX, move.clientY));
        setDraft({ ring: [...latestCorners.current, at], isOpen: true });
      };
      window.addEventListener("pointermove", onMove);
      stopFollowing.current = () => window.removeEventListener("pointermove", onMove);
      return;
    }

    setDraft({ ring: [start, start], isOpen: false });
    follow(
      event,
      view,
      (at) => setDraft({ ring: ringOf(shapeFrom(kind, [start, at])), isOpen: false }),
      (at) => addShape(at ? shapeFrom(kind, [start, at]) : placedShape(kind, start), paper),
    );
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
      follow(
        event,
        view,
        (at) => setPreview({ id: selected.id, shape: withHandleMoved(selected.shape, handle, at) }),
        (at) => at && changeShape(selected.id, withHandleMoved(selected.shape, handle, at), paper),
      );
      return true;
    }

    const id = highlightAt(paper, pressed);
    const hit = latest.current.find((highlight) => highlight.id === id);
    if (!hit) {
      setSelectedId(null);
      setEditingId(null);
      return false;
    }
    event.preventDefault();
    setSelectedId(hit.id);
    setEditingId(null);
    const toRd = paperToRd(paper);
    const start = toRd?.(pressed);
    if (!start) return true;
    follow(
      event,
      view,
      (at) => setPreview({ id: hit.id, shape: moved(hit.shape, at.x - start.x, at.y - start.y) }),
      (at) => at && changeShape(hit.id, moved(hit.shape, at.x - start.x, at.y - start.y), paper),
    );
    return true;
  };

  const onPaperPointerDown = (event: React.PointerEvent, view: PaperView): boolean => {
    if (isDrawing) {
      event.preventDefault();
      pressWhileDrawing(event, view);
      return true;
    }
    if (!isAvailable) return false;
    return pressToEdit(event, view);
  };

  const finishPolygon = () => {
    // A double-click lands two presses on the last corner; keep one.
    const kept = corners.filter((point, index) => index === 0 || Math.hypot(point.x - corners[index - 1].x, point.y - corners[index - 1].y) > 0.05);
    if (kept.length < 3) {
      onRefused("A polygon needs at least three corners.");
      return;
    }
    addShape(shapeFrom("polygon", kept), paperOf());
  };

  const onPaperDoubleClick = (): boolean => {
    if (!isDrawing || kind !== "polygon") return false;
    finishPolygon();
    return true;
  };

  const startDrawing = (next: HighlightKind) => {
    if (!isAvailable) {
      onRefused(unavailable);
      return;
    }
    setKind(next);
    setCorners([]);
    setDraft(null);
    setSelectedId(null);
    setDrawing(true);
  };

  /** A shape of a house's size in the middle of the view, for the keyboard. */
  const placeInView = (next: HighlightKind) => {
    const paper = paperOf();
    const centre = canvas.current?.centreMm();
    const toRd = paperToRd(paper);
    if (!isAvailable || !centre || !toRd) {
      onRefused(unavailable);
      return;
    }
    setKind(next);
    addShape(placedShape(next, toRd(centre)), paper);
  };

  const remove = (id: string) => {
    onHighlights(withoutHighlight(latest.current, id));
    setSelectedId(null);
    setEditingId(null);
  };

  const changeSelected = (change: (shape: HighlightShape, toRd: (point: Point) => Point) => HighlightShape) => {
    const paper = paperOf();
    const toRd = paperToRd(paper);
    const selected = latest.current.find((highlight) => highlight.id === selectedId);
    if (selected && toRd) changeShape(selected.id, change(selected.shape, toRd), paper);
  };

  usePlainKeys((key, event) => {
    if (key.toLowerCase() === HIGHLIGHT_KEY && !event.repeat) {
      if (isDrawing) stopDrawing();
      else startDrawing(kind);
      return;
    }
    if (isDrawing) {
      if (key === "Escape") stopDrawing();
      if (key === "Enter" && kind === "polygon") finishPolygon();
      if (key === "Backspace" && kind === "polygon") {
        const next = corners.slice(0, -1);
        setCorners(next);
        setDraft(next.length > 0 ? { ring: next, isOpen: true } : null);
        if (next.length === 0) stopFollowing.current?.();
      }
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
    const step = event.shiftKey ? NUDGE_LARGE_MM : NUDGE_MM;
    const moves: Record<string, [number, number]> = { ArrowLeft: [-step, 0], ArrowRight: [step, 0], ArrowUp: [0, -step], ArrowDown: [0, step] };
    const move = moves[key];
    if (move) {
      event.preventDefault();
      changeSelected((shape, toRd) => {
        const [from, to] = [toRd({ x: 0, y: 0 }), toRd({ x: move[0], y: move[1] })];
        return moved(shape, to.x - from.x, to.y - from.y);
      });
    }
  }, isEnabled && editingId === null);

  return {
    kind,
    isDrawing,
    isAvailable,
    draft,
    preview,
    selectedId,
    editingId,
    setSelectedId,
    setEditingId,
    onPaperPointerDown,
    onPaperDoubleClick,
    startDrawing,
    stopDrawing,
    placeInView,
    remove,
  };
}

function isNear(a: Point, b: Point): boolean {
  return Math.hypot(a.x - b.x, a.y - b.y) <= HANDLE_HIT_MM;
}

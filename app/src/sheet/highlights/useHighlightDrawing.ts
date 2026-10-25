// Drawing a new highlight. A rectangle is dragged from corner to corner and a
// circle from its centre outwards; a click instead places one of a house's
// size there. A polygon takes a click per corner and closes on a
// double-click, Return or a click on its first corner.

import { useEffect, useRef, useState, type RefObject } from "react";
import type { Highlight, HighlightKind, HighlightShape } from "../../lib/types";
import type { CanvasControls, PaperView } from "../Canvas";
import { useLiveValue } from "../liveValue";
import { paperToRd, rdToPaper, type Point } from "../paper";
import { newHighlight, nextHighlightId, placedShape, ringOf, shapeFrom, withHighlight } from "./highlightEdits";
import { followOnGround, isNear, paperOf, problemWith } from "./highlightGround";

export const UNAVAILABLE = "Highlights are drawn and edited on a plan, axonometric or explode view.";
// Two presses closer than this, in metres, are the double-click that ends a polygon, not two corners.
const SAME_CORNER_M = 0.05;
const MIN_POLYGON_CORNERS = 3;

/** What the overlay draws while the pointer works: RD New points, closed or still being drawn. */
export type HighlightDraft = { ring: Point[]; isOpen: boolean };

type Options = {
  /** Always the current list, read when a shape is added. */
  latest: RefObject<Highlight[]>;
  onHighlights: (next: Highlight[], mergeKey?: string) => void;
  canvas: RefObject<CanvasControls | null>;
  isAvailable: boolean;
  isEnabled: boolean;
  /** Selects the new highlight and opens its label for typing. */
  onAdded: (id: string) => void;
  /** Lets go of the selected highlight, since drawing starts. */
  onStart: () => void;
  onRefused: (message: string) => void;
};

export function useHighlightDrawing({ latest, onHighlights, canvas, isAvailable, isEnabled, onAdded, onStart, onRefused }: Options) {
  const [kind, setKind] = useState<HighlightKind>("rectangle");
  const [isDrawing, setDrawing] = useState(false);
  const [corners, setCorners] = useState<Point[]>([]);
  const draft = useLiveValue<HighlightDraft | null>(null);
  // While a polygon is drawn, how to stop its next side following the pointer.
  const stopFollowing = useRef<(() => void) | null>(null);
  const latestCorners = useRef(corners);
  latestCorners.current = corners;

  useEffect(() => {
    if (!isAvailable || !isEnabled) stopDrawing();
  }, [isAvailable, isEnabled]);

  function stopDrawing() {
    stopFollowing.current?.();
    stopFollowing.current = null;
    setDrawing(false);
    setCorners([]);
    draft.set(null);
  }

  const addShape = (shape: HighlightShape, paper: Element | null) => {
    const problem = problemWith(shape, paper);
    if (problem) {
      onRefused(problem);
      draft.set(null);
      return;
    }
    const id = nextHighlightId(latest.current);
    // Drawing and typing the first label merge into one undo step.
    onHighlights(withHighlight(latest.current, newHighlight(id, shape)), `highlight:${id}`);
    stopDrawing();
    onAdded(id);
  };

  const press = (event: React.PointerEvent, view: PaperView) => {
    const paper = view.getPaper();
    const toRd = paperToRd(paper);
    if (!toRd) {
      onRefused(UNAVAILABLE);
      return;
    }
    const pressed = view.toMm(event.clientX, event.clientY);
    if (kind === "polygon") {
      addCorner(view, pressed, toRd);
      return;
    }

    const start = toRd(pressed);
    draft.set({ ring: [start, start], isOpen: false });
    followOnGround(
      event,
      view,
      (at) => draft.set({ ring: ringOf(shapeFrom(kind, [start, at])), isOpen: false }),
      (at) => addShape(at ? shapeFrom(kind, [start, at]) : placedShape(kind, start), paper),
    );
  };

  /** A polygon's next corner, or the polygon closed when the press is on its first one. */
  const addCorner = (view: PaperView, pressed: Point, toRd: (point: Point) => Point) => {
    const paper = view.getPaper();
    const first = corners[0] && rdToPaper(paper)?.(corners[0]);
    if (first && isNear(first, pressed) && corners.length >= MIN_POLYGON_CORNERS) {
      addShape(shapeFrom("polygon", corners), paper);
      return;
    }
    const next = [...corners, toRd(pressed)];
    setCorners(next);
    draft.set({ ring: next, isOpen: true });
    // The next side follows the pointer until the next click.
    stopFollowing.current?.();
    const onMove = (move: PointerEvent) => {
      draft.set({ ring: [...latestCorners.current, toRd(view.toMm(move.clientX, move.clientY))], isOpen: true });
    };
    window.addEventListener("pointermove", onMove);
    stopFollowing.current = () => window.removeEventListener("pointermove", onMove);
  };

  const finishPolygon = () => {
    // A double-click lands two presses on the last corner; keep one.
    const kept = corners.filter((point, index) => index === 0 || Math.hypot(point.x - corners[index - 1].x, point.y - corners[index - 1].y) > SAME_CORNER_M);
    if (kept.length < MIN_POLYGON_CORNERS) {
      onRefused("A polygon needs at least three corners.");
      return;
    }
    addShape(shapeFrom("polygon", kept), paperOf());
  };

  const dropLastCorner = () => {
    const next = corners.slice(0, -1);
    setCorners(next);
    draft.set(next.length > 0 ? { ring: next, isOpen: true } : null);
    if (next.length === 0) stopFollowing.current?.();
  };

  const startDrawing = (next: HighlightKind) => {
    if (!isAvailable) {
      onRefused(UNAVAILABLE);
      return;
    }
    setKind(next);
    setCorners([]);
    draft.set(null);
    onStart();
    setDrawing(true);
  };

  /** A shape of a house's size in the middle of the view, for the keyboard. */
  const placeInView = (next: HighlightKind) => {
    const paper = paperOf();
    const centre = canvas.current?.centreMm();
    const toRd = paperToRd(paper);
    if (!isAvailable || !centre || !toRd) {
      onRefused(UNAVAILABLE);
      return;
    }
    setKind(next);
    addShape(placedShape(next, toRd(centre)), paper);
  };

  return { kind, isDrawing, draft, press, finishPolygon, dropLastCorner, startDrawing, stopDrawing, placeInView };
}

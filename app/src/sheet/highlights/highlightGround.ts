// Where the pointer is on the ground while a highlight is drawn or dragged,
// and whether a shape may be kept there. Both the drawing and the editing
// half of the highlight tool use these.

import { followPointer } from "../../lib/pointerDrag";
import type { HighlightShape } from "../../lib/types";
import type { PaperView } from "../Canvas";
import { isOutsideSite, paperToRd, rdToPaper, type Point } from "../paper";
import { areaOf, middleOf, MIN_AREA_M2 } from "./highlightEdits";

// How close to a handle or a polygon's first corner, in paper millimetres, a press still takes it.
const HANDLE_HIT_MM = 2.5;

/** The sheet's drawn paper, for a change that does not start from a press on it. */
export function paperOf() {
  return document.querySelector(".paper");
}

/** Follows the pointer from a press until it is let go, telling `onDrag` where it is in RD New. */
export function followOnGround(event: React.PointerEvent, view: PaperView, onDrag: (at: Point) => void, onDrop: (at: Point | null) => void) {
  const toRd = paperToRd(view.getPaper());
  if (!toRd) return;
  const at = (pointer: PointerEvent) => toRd(view.toMm(pointer.clientX, pointer.clientY));
  followPointer(event, {
    onDrag: (move) => onDrag(at(move)),
    onDrop: (up, isDragged) => onDrop(isDragged ? at(up) : null),
  });
}

/** Why a shape cannot be kept, or null when it can. */
export function problemWith(shape: HighlightShape, paper: Element | null): string | null {
  if (areaOf(shape) < MIN_AREA_M2) return `A highlight needs at least ${MIN_AREA_M2} m². Draw it a little larger.`;
  const middle = rdToPaper(paper)?.(middleOf(shape));
  if (middle && isOutsideSite(paper, middle)) {
    return "The middle of a highlight goes inside the outline of the sheet's area.";
  }
  return null;
}

/** Whether two paper points, in millimetres, are close enough for a press on one to take the other. */
export function isNear(a: Point, b: Point): boolean {
  return Math.hypot(a.x - b.x, a.y - b.y) <= HANDLE_HIT_MM;
}

// The View diagram's eye on the rendered sheet, read from the SVG the core
// drew, and small, pure edits to a look's viewpoint. Every edit returns a new
// look, so each one goes straight onto the undo stack.

import type { Diagram, Look, Pin } from "../../lib/types";
import type { Point } from "../pins/sheetPins";

// The core writes the eye it lets the app drag as one group with its paper position in data attributes.
const EYE_SELECTOR = "#view-point";
// The layer only the View diagram draws, crates/core/src/scene.rs.
const VIEWSHED_LAYER = "viewshed";

/** Whether this diagram, as drawn, has an eye to place. */
export function drawsView(diagram: Diagram | undefined): boolean {
  return Boolean(diagram?.layers.includes(VIEWSHED_LAYER));
}

/** Where the core drew the eye, in paper millimetres, or null when it drew none. */
export function readEye(paper: Element | null): Point | null {
  const group = paper?.querySelector<SVGGElement>(EYE_SELECTOR);
  if (!group) return null;
  const x = Number(group.getAttribute("data-x"));
  const y = Number(group.getAttribute("data-y"));
  if (!Number.isFinite(x) || !Number.isFinite(y)) return null;
  return { x, y };
}

/** The eye at an RD New point, or back at the site centre for `null`. */
export function withViewPoint(look: Look, point: Point | null): Look {
  return { ...look, view_point: point ? [point.x, point.y] : null };
}

/** The pin the eye stands on, when it was put there from the menu and the pin has not moved since. */
export function pinUnderEye(look: Look, pins: Pin[]): Pin | undefined {
  const point = look.view_point;
  if (!point) return undefined;
  return pins.find((pin) => pin.x === point[0] && pin.y === point[1]);
}

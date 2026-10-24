// The section line on the rendered sheet, read from the SVG the core drew, and
// small, pure edits to a look's line. Every edit returns a new look, so each
// one goes straight onto the undo stack.

import type { Look, SectionLine } from "../../lib/types";
import type { Point } from "../pins/sheetPins";

// The core's defaults and limits, from crates/core/src/section.rs.
export const DEFAULT_DEPTH_M = 60;
export const MAX_DEPTH_M = 500;
export const MIN_LENGTH_M = 5;
// The core writes the marker as one group with its ends on paper in data attributes.
const MARKER_SELECTOR = "#section-line";

export type PaperEnds = { start: Point; end: Point };

/** Where the core drew the line's ends, in paper millimetres, or null when it drew none. */
export function readEnds(paper: Element | null): PaperEnds | null {
  const group = paper?.querySelector<SVGGElement>(MARKER_SELECTOR);
  if (!group) return null;
  const read = (name: string) => Number(group.getAttribute(`data-${name}`));
  const values = [read("start-x"), read("start-y"), read("end-x"), read("end-y")];
  if (values.some((value) => !Number.isFinite(value))) return null;
  return { start: { x: values[0], y: values[1] }, end: { x: values[2], y: values[3] } };
}

/** A new line from A to A', keeping the depth of the one it replaces. */
export function withSection(look: Look, start: Point, end: Point): Look {
  const depth_m = look.section?.depth_m ?? DEFAULT_DEPTH_M;
  return { ...look, section: { start: [start.x, start.y], end: [end.x, end.y], depth_m } };
}

export function withSectionDepth(look: Look, depth_m: number): Look {
  if (!look.section) return look;
  return { ...look, section: { ...look.section, depth_m } };
}

/** A and A' swapped, so the section looks the other way. */
export function withSectionFlipped(look: Look): Look {
  if (!look.section) return look;
  const { start, end } = look.section;
  return { ...look, section: { ...look.section, start: end, end: start } };
}

/** No line; a sheet showing the section goes back to its plan, which is what the core can still draw. */
export function withoutSection(look: Look): Look {
  const view = look.view.mode === "section" ? { ...look.view, mode: "plan" as const } : look.view;
  return { ...look, section: null, view };
}

export function lengthOf(line: SectionLine): number {
  return Math.hypot(line.end[0] - line.start[0], line.end[1] - line.start[1]);
}

/** Distance on paper from `point` to the segment between `a` and `b`. */
export function distanceToSegment(point: Point, a: Point, b: Point): number {
  const [dx, dy] = [b.x - a.x, b.y - a.y];
  const squared = dx * dx + dy * dy;
  const share = squared === 0 ? 0 : Math.max(0, Math.min(1, ((point.x - a.x) * dx + (point.y - a.y) * dy) / squared));
  return Math.hypot(point.x - (a.x + share * dx), point.y - (a.y + share * dy));
}

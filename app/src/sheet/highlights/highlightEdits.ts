// Changes to the project's highlights, and the shape sums the tool needs, all
// in RD New metres. Every edit returns a new list and leaves the old one alone,
// so each result can go straight onto the undo stack.

import type { Highlight, HighlightKind, HighlightShape } from "../../lib/types";
import type { Point } from "../paper";

// The shapes in the order the bar offers them, in one place; crates/core/src/highlight.rs has the same list.
export const HIGHLIGHT_KINDS: { kind: HighlightKind; label: string; drawTip: string }[] = [
  { kind: "rectangle", label: "Rectangle", drawTip: "drag from corner to corner" },
  { kind: "circle", label: "Circle", drawTip: "drag from the centre outwards" },
  { kind: "polygon", label: "Polygon", drawTip: "click each corner, double-click or Return to finish" },
];

// The name of a highlight without a label, as the core's legend says it.
export const UNLABELLED = "Highlighted area";
// The same limit as the core's MIN_AREA_M2: less is a slip of the mouse.
export const MIN_AREA_M2 = 4;
// What a highlight is drawn in until the user picks a colour, as in crates/core/src/highlight.rs.
export const DEFAULT_COLOUR = "@accent";
// A shape placed from the keyboard, before it is reshaped: about a house on its plot.
const PLACED_HALF_M = 10;
// Enough segments that a drafted circle looks round.
const CIRCLE_SEGMENTS = 64;

/** The first "highlight-N" id no highlight uses yet, as the core picks it. */
export function nextHighlightId(highlights: Highlight[]): string {
  let number = highlights.length + 1;
  while (highlights.some((highlight) => highlight.id === `highlight-${number}`)) number += 1;
  return `highlight-${number}`;
}

export function withHighlight(highlights: Highlight[], highlight: Highlight): Highlight[] {
  return [...highlights, highlight];
}

export function withChangedHighlight(highlights: Highlight[], id: string, patch: Partial<Omit<Highlight, "id">>): Highlight[] {
  return highlights.map((highlight) => (highlight.id === id ? { ...highlight, ...patch } : highlight));
}

export function withoutHighlight(highlights: Highlight[], id: string): Highlight[] {
  return highlights.filter((highlight) => highlight.id !== id);
}

/** A fresh highlight with the plainest style: a solid outline in the accent, nothing dimmed, no area. */
export function newHighlight(id: string, shape: HighlightShape): Highlight {
  return { id, label: "", shape, outline: "solid", colour: DEFAULT_COLOUR, dims_rest: false, shows_area: false };
}

/** The shape a drag from `start` to `end` draws, or a polygon through `points`. */
export function shapeFrom(kind: HighlightKind, points: Point[]): HighlightShape {
  const [start, end] = [points[0], points[points.length - 1]];
  if (kind === "circle") {
    return { kind, center: [start.x, start.y], radius_m: Math.hypot(end.x - start.x, end.y - start.y) };
  }
  if (kind === "rectangle") {
    return {
      kind,
      min: [Math.min(start.x, end.x), Math.min(start.y, end.y)],
      max: [Math.max(start.x, end.x), Math.max(start.y, end.y)],
    };
  }
  return { kind, points: points.map((point) => [point.x, point.y]) };
}

/** A shape of `kind` around `middle`, for placing one without a pointer. */
export function placedShape(kind: HighlightKind, middle: Point): HighlightShape {
  const corners = [
    { x: middle.x - PLACED_HALF_M, y: middle.y - PLACED_HALF_M },
    { x: middle.x + PLACED_HALF_M, y: middle.y - PLACED_HALF_M },
    { x: middle.x, y: middle.y + PLACED_HALF_M },
  ];
  if (kind === "circle") return shapeFrom(kind, [middle, { x: middle.x + PLACED_HALF_M, y: middle.y }]);
  if (kind === "rectangle") return shapeFrom(kind, [corners[0], { x: middle.x + PLACED_HALF_M, y: middle.y + PLACED_HALF_M }]);
  return shapeFrom(kind, corners);
}

/** The outline as RD New points, a circle as many. Not closed. */
export function ringOf(shape: HighlightShape): Point[] {
  if (shape.kind === "circle") {
    const [x, y] = shape.center;
    return Array.from({ length: CIRCLE_SEGMENTS }, (_, index) => {
      const angle = (2 * Math.PI * index) / CIRCLE_SEGMENTS;
      return { x: x + shape.radius_m * Math.cos(angle), y: y + shape.radius_m * Math.sin(angle) };
    });
  }
  if (shape.kind === "rectangle") {
    const [[x1, y1], [x2, y2]] = [shape.min, shape.max];
    return [
      { x: x1, y: y1 },
      { x: x2, y: y1 },
      { x: x2, y: y2 },
      { x: x1, y: y2 },
    ];
  }
  return shape.points.map(([x, y]) => ({ x, y }));
}

/** The points a selected shape can be dragged by: a rectangle's corners, a circle's edge east, north, west and south, a polygon's corners. */
export function handlesOf(shape: HighlightShape): Point[] {
  if (shape.kind === "circle") {
    const [x, y] = shape.center;
    const r = shape.radius_m;
    return [
      { x: x + r, y },
      { x, y: y + r },
      { x: x - r, y },
      { x, y: y - r },
    ];
  }
  return ringOf(shape);
}

/** The shape with handle `index` dragged to `to`. A rectangle keeps the opposite corner where it was. */
export function withHandleMoved(shape: HighlightShape, index: number, to: Point): HighlightShape {
  if (shape.kind === "circle") {
    return { ...shape, radius_m: Math.hypot(to.x - shape.center[0], to.y - shape.center[1]) };
  }
  if (shape.kind === "rectangle") {
    const opposite = ringOf(shape)[(index + 2) % 4];
    return shapeFrom("rectangle", [opposite, to]);
  }
  return { ...shape, points: shape.points.map((point, at) => (at === index ? [to.x, to.y] : point)) };
}

export function moved(shape: HighlightShape, dx: number, dy: number): HighlightShape {
  if (shape.kind === "circle") return { ...shape, center: [shape.center[0] + dx, shape.center[1] + dy] };
  if (shape.kind === "rectangle") {
    return { ...shape, min: [shape.min[0] + dx, shape.min[1] + dy], max: [shape.max[0] + dx, shape.max[1] + dy] };
  }
  return { ...shape, points: shape.points.map(([x, y]) => [x + dx, y + dy]) };
}

/** The shape grown or shrunk by `factor` about its middle. */
export function scaled(shape: HighlightShape, factor: number): HighlightShape {
  const middle = middleOf(shape);
  const away = ({ x, y }: Point): [number, number] => [middle.x + (x - middle.x) * factor, middle.y + (y - middle.y) * factor];
  if (shape.kind === "circle") return { ...shape, radius_m: shape.radius_m * factor };
  if (shape.kind === "rectangle") {
    return { ...shape, min: away({ x: shape.min[0], y: shape.min[1] }), max: away({ x: shape.max[0], y: shape.max[1] }) };
  }
  return { ...shape, points: shape.points.map(([x, y]) => away({ x, y })) };
}

/** In square metres; a circle's exactly. */
export function areaOf(shape: HighlightShape): number {
  if (shape.kind === "circle") return Math.PI * shape.radius_m ** 2;
  return Math.abs(signedArea(ringOf(shape)));
}

/** A circle's centre, otherwise the centroid, the point the core checks against the site. */
export function middleOf(shape: HighlightShape): Point {
  if (shape.kind === "circle") return { x: shape.center[0], y: shape.center[1] };
  const ring = ringOf(shape);
  const twiceArea = 2 * signedArea(ring);
  if (twiceArea === 0) return ring[0];
  let [x, y] = [0, 0];
  ring.forEach((point, index) => {
    const next = ring[(index + 1) % ring.length];
    const cross = point.x * next.y - next.x * point.y;
    x += (point.x + next.x) * cross;
    y += (point.y + next.y) * cross;
  });
  return { x: x / (3 * twiceArea), y: y / (3 * twiceArea) };
}

function signedArea(ring: Point[]): number {
  let sum = 0;
  ring.forEach((point, index) => {
    const next = ring[(index + 1) % ring.length];
    sum += point.x * next.y - next.x * point.y;
  });
  return sum / 2;
}

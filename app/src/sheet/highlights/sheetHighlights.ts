// The highlights on the rendered sheet, read from the SVG the core drew, and
// the sum that puts a point in RD New metres back on the paper.
//
// The core writes each highlight's shape as a path carrying its id in
// `data-highlight` (crates/core/src/render/highlights.rs), so a click is
// tested against exactly what is drawn. Handles and drafts are drawn by the
// app over the sheet, so they need RD New -> paper, the other way round from
// the pins' `paperToRd`. The ground points the core writes are 1 km east and
// 1 km north of the first one, so each pair gives one direction on paper.

import type { Point } from "../pins/sheetPins";

// How close to an outline, in paper millimetres, a press still picks it up.
const EDGE_HIT_MM = 1.5;

/** RD New metres to paper millimetres, or null when the sheet carries no ground points. */
export function rdToPaper(paper: Element | null): ((point: Point) => Point) | null {
  const values = paper?.querySelector("#paper")?.getAttribute("data-ground-points")?.split(" ").map(Number);
  if (!values || values.length !== 12 || values.some((value) => !Number.isFinite(value))) return null;
  const [px0, py0, rx0, ry0, px1, py1, rx1, , px2, py2, , ry2] = values;
  const [eastX, eastY] = [(px1 - px0) / (rx1 - rx0), (py1 - py0) / (rx1 - rx0)];
  const [northX, northY] = [(px2 - px0) / (ry2 - ry0), (py2 - py0) / (ry2 - ry0)];

  return ({ x, y }) => ({
    x: px0 + (x - rx0) * eastX + (y - ry0) * northX,
    y: py0 + (x - rx0) * eastY + (y - ry0) * northY,
  });
}

/** The highlight under a paper point, the one drawn last when several overlap. */
export function highlightAt(paper: Element | null, point: Point): string | null {
  // An explode view draws each highlight on every plate, but only the top plate lies on the sheet's ground.
  const paths = paper ? [...paper.querySelectorAll<SVGPathElement>("path[data-highlight]")].filter(isOnGround) : [];
  const hits = paths.filter((path) => path.isPointInFill(svgPoint(path, point)) || isNearEdge(path, svgPoint(path, point)));
  return hits[hits.length - 1]?.getAttribute("data-highlight") ?? null;
}

function isOnGround(path: SVGPathElement): boolean {
  const plate = path.closest("[data-plate]");
  return plate === null || plate.getAttribute("data-index") === "0";
}

/** Near the outline counts too, so a thin ring of a highlight can be picked up by its edge. */
function isNearEdge(path: SVGPathElement, at: DOMPointInit): boolean {
  const width = path.style.strokeWidth;
  path.style.strokeWidth = `${2 * EDGE_HIT_MM}`;
  const isNear = path.isPointInStroke(at);
  path.style.strokeWidth = width;
  return isNear;
}

/** A point the SVG hit tests take in every engine: Chromium wants an SVGPoint where WebKit also takes a DOMPoint. */
export function svgPoint(element: SVGGraphicsElement, { x, y }: Point): DOMPointInit {
  const point = element.ownerSVGElement?.createSVGPoint();
  if (!point) return new DOMPoint(x, y);
  point.x = x;
  point.y = y;
  return point;
}

// The highlights on the rendered sheet, read from the SVG the core drew.
//
// The core writes each highlight's shape as a path carrying its id in
// `data-highlight` (crates/core/src/render/highlights.rs), so a click is
// tested against exactly what is drawn.

import { svgPoint, type Point } from "../paper";

// How close to an outline, in paper millimetres, a press still picks it up.
const EDGE_HIT_MM = 1.5;

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

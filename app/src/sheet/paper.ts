// The paper and the ground: points on the rendered sheet in millimetres, and
// the sums between them and RD New metres.
//
// The core puts three ground points on the paper rectangle
// (`#paper[data-ground-points]`): where each lands on paper and where it is in
// RD New. The ground maps onto the paper linearly in every view, so those
// three fix the map both ways for a plan, a tilted view and the top plate of
// an explode. The SVG's user unit is the millimetre, so boxes read off it are
// in paper millimetres too.

export type Point = { x: number; y: number };
export type Box = { x: number; y: number; width: number; height: number };

// Small marks are hard to hit exactly; this much paper around each one counts too.
const HIT_SLOP_MM = 1.5;
// RD New positions are kept to the centimetre, like the scene.
const CENTIMETRES = 100;
// Four numbers for each of the three points: paper x and y, then RD New x and y.
const GROUND_POINT_VALUES = 12;
// The outline of the area the sheet draws, which the core writes in plan and axonometric views.
const BOUNDARY_SELECTOR = "#site-boundary";

/** The affine map from paper to ground: rd = [a b; c d]·(paper − p0) + r0. */
type GroundMap = { p0: Point; r0: Point; a: number; b: number; c: number; d: number };

function groundMap(paper: Element | null): GroundMap | null {
  const values = paper?.querySelector("#paper")?.getAttribute("data-ground-points")?.split(" ").map(Number);
  if (!values || values.length !== GROUND_POINT_VALUES || values.some((value) => !Number.isFinite(value))) return null;
  const [px0, py0, rx0, ry0, px1, py1, rx1, ry1, px2, py2, rx2, ry2] = values;

  // Solve the matrix from the two other point pairs.
  const [ux, uy, vx, vy] = [px1 - px0, py1 - py0, px2 - px0, py2 - py0];
  const determinant = ux * vy - vx * uy;
  if (determinant === 0) return null;
  const [i00, i01, i10, i11] = [vy / determinant, -vx / determinant, -uy / determinant, ux / determinant];
  const [dux, duy, dvx, dvy] = [rx1 - rx0, ry1 - ry0, rx2 - rx0, ry2 - ry0];

  return {
    p0: { x: px0, y: py0 },
    r0: { x: rx0, y: ry0 },
    a: dux * i00 + dvx * i10,
    b: dux * i01 + dvx * i11,
    c: duy * i00 + dvy * i10,
    d: duy * i01 + dvy * i11,
  };
}

/** Paper millimetres to RD New metres, or null when the sheet carries no ground points. */
export function paperToRd(paper: Element | null): ((point: Point) => Point) | null {
  const map = groundMap(paper);
  if (!map) return null;
  const { p0, r0, a, b, c, d } = map;

  return ({ x, y }) => ({
    x: roundCm(r0.x + a * (x - p0.x) + b * (y - p0.y)),
    y: roundCm(r0.y + c * (x - p0.x) + d * (y - p0.y)),
  });
}

/** RD New metres to paper millimetres, the inverse of `paperToRd`, or null when the sheet carries no ground points. */
export function rdToPaper(paper: Element | null): ((point: Point) => Point) | null {
  const map = groundMap(paper);
  if (!map) return null;
  const { p0, r0, a, b, c, d } = map;
  const determinant = a * d - b * c;
  if (determinant === 0) return null;

  return ({ x, y }) => ({
    x: p0.x + (d * (x - r0.x) - b * (y - r0.y)) / determinant,
    y: p0.y + (a * (y - r0.y) - c * (x - r0.x)) / determinant,
  });
}

/** The item whose box holds a paper point, the one drawn last when several overlap. */
export function boxAt<T extends { box: Box }>(items: T[], x: number, y: number): T | null {
  const hits = items.filter(({ box }) => {
    const isInsideX = x >= box.x - HIT_SLOP_MM && x <= box.x + box.width + HIT_SLOP_MM;
    const isInsideY = y >= box.y - HIT_SLOP_MM && y <= box.y + box.height + HIT_SLOP_MM;
    return isInsideX && isInsideY;
  });
  return hits[hits.length - 1] ?? null;
}

/** The paper the sheet sits on, and the millimetre under a screen point, for drops from outside the canvas. */
export function paperPointAt(clientX: number, clientY: number): { paper: Element; point: Point } | null {
  const paper = document.querySelector(".paper");
  const svg = paper?.querySelector("svg");
  const viewBox = svg?.viewBox.baseVal;
  if (!paper || !svg || !viewBox || viewBox.width === 0) return null;
  const rect = svg.getBoundingClientRect();
  const isOnPaper = clientX >= rect.left && clientX <= rect.right && clientY >= rect.top && clientY <= rect.bottom;
  if (!isOnPaper) return null;
  const mmPerPx = viewBox.width / rect.width;
  return { paper, point: { x: (clientX - rect.left) * mmPerPx, y: (clientY - rect.top) * mmPerPx } };
}

/** Whether a paper point lies outside the sheet's area; false when the view draws no outline. */
export function isOutsideSite(paper: Element | null, point: Point): boolean {
  const boundary = paper?.querySelector<SVGPathElement>(BOUNDARY_SELECTOR);
  return Boolean(boundary && !boundary.isPointInFill(svgPoint(boundary, point)));
}

/** A point the SVG hit tests take in every engine: Chromium wants an SVGPoint where WebKit also takes a DOMPoint. */
export function svgPoint(element: SVGGraphicsElement, { x, y }: Point): DOMPointInit {
  const point = element.ownerSVGElement?.createSVGPoint();
  if (!point) return new DOMPoint(x, y);
  point.x = x;
  point.y = y;
  return point;
}

const roundCm = (metres: number) => Math.round(metres * CENTIMETRES) / CENTIMETRES;

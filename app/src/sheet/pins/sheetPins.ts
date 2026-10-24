// The pins on the rendered sheet, read from the SVG the core drew, and the
// sum that turns a point on the paper into RD New metres.
//
// The core writes each pin as a group with its id, icon, label and paper
// anchor in data attributes, as it does for sheet text, and puts three ground
// points on the paper rectangle: where they land on paper and where they are
// in RD New. The ground maps onto the paper linearly in every view, so those
// three fix the sum for a plan, a tilted view and the top plate of an explode.

import type { PinIcon } from "../../lib/types";
import type { Box } from "../text/sheetTexts";

export type SheetPin = {
  id: string;
  icon: PinIcon;
  label: string;
  /** The anchor in paper millimetres: the point the marker stands on. */
  x: number;
  y: number;
  box: Box;
};

export type Point = { x: number; y: number };

// Markers are small; this much paper around each one counts as a hit too.
const HIT_SLOP_MM = 1.5;
// RD New positions are kept to the centimetre, like the scene.
const CENTIMETRES = 100;

export function readPins(paper: Element | null): SheetPin[] {
  const groups = paper ? [...paper.querySelectorAll<SVGGElement>("g[data-pin]")] : [];
  return groups.map((group) => {
    const data = group.dataset;
    const bbox = group.getBBox();
    return {
      id: data.pin ?? "",
      icon: (data.icon as PinIcon | undefined) ?? "pin",
      label: data.label ?? "",
      x: Number(data.x),
      y: Number(data.y),
      box: { x: bbox.x, y: bbox.y, width: bbox.width, height: bbox.height },
    };
  });
}

/** The pin under a point on the paper, the one drawn last when several overlap. */
export function pinAt(pins: SheetPin[], x: number, y: number): SheetPin | null {
  const hits = pins.filter(({ box }) => {
    const isInsideX = x >= box.x - HIT_SLOP_MM && x <= box.x + box.width + HIT_SLOP_MM;
    const isInsideY = y >= box.y - HIT_SLOP_MM && y <= box.y + box.height + HIT_SLOP_MM;
    return isInsideX && isInsideY;
  });
  return hits[hits.length - 1] ?? null;
}

export function pinGroup(paper: Element | null, id: string): SVGGElement | null {
  return paper?.querySelector<SVGGElement>(`g[data-pin="${CSS.escape(id)}"]`) ?? null;
}

/** Paper millimetres to RD New metres, or null when the sheet carries no ground points. */
export function paperToRd(paper: Element | null): ((point: Point) => Point) | null {
  const values = paper?.querySelector("#paper")?.getAttribute("data-ground-points")?.split(" ").map(Number);
  if (!values || values.length !== 12 || values.some((value) => !Number.isFinite(value))) return null;
  const [px0, py0, rx0, ry0, px1, py1, rx1, ry1, px2, py2, rx2, ry2] = values;

  // Solve rd = A·(paper − p0) + r0 from the two other point pairs.
  const [ux, uy, vx, vy] = [px1 - px0, py1 - py0, px2 - px0, py2 - py0];
  const determinant = ux * vy - vx * uy;
  if (determinant === 0) return null;
  const [i00, i01, i10, i11] = [vy / determinant, -vx / determinant, -uy / determinant, ux / determinant];
  const [dux, duy, dvx, dvy] = [rx1 - rx0, ry1 - ry0, rx2 - rx0, ry2 - ry0];
  const a = dux * i00 + dvx * i10;
  const b = dux * i01 + dvx * i11;
  const c = duy * i00 + dvy * i10;
  const d = duy * i01 + dvy * i11;

  return ({ x, y }) => ({
    x: roundCm(rx0 + a * (x - px0) + b * (y - py0)),
    y: roundCm(ry0 + c * (x - px0) + d * (y - py0)),
  });
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

const roundCm = (metres: number) => Math.round(metres * CENTIMETRES) / CENTIMETRES;

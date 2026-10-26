// The sums behind moving and turning the model on the sheet. A placement is a
// point in RD New metres and a clockwise turn, as the core keeps it in
// crates/core/src/model.rs, and the footprint arrives in the model's own frame.

import type { ModelView, Placement } from "../../lib/types";
import type { Point } from "../paper";

const FULL_TURN_DEG = 360;

/** The model's footprint outlines in RD New metres where `placement` stands it. */
export function footprintOnSite(model: ModelView, placement: Placement): Point[][] {
  const angle = (placement.rotation_deg * Math.PI) / 180;
  const cos = Math.cos(angle);
  const sin = Math.sin(angle);
  return model.footprint.map((ring) =>
    ring.map(([x, y]) => ({
      x: placement.x + x * cos + y * sin,
      y: placement.y - x * sin + y * cos,
    })),
  );
}

/** The bearing from `centre` to `point`, in degrees clockwise from north. */
export function bearingDeg(centre: Point, point: Point): number {
  return (Math.atan2(point.x - centre.x, point.y - centre.y) * 180) / Math.PI;
}

/** A turn kept between 0 and 360 degrees. */
export function normalisedTurn(degrees: number): number {
  return ((degrees % FULL_TURN_DEG) + FULL_TURN_DEG) % FULL_TURN_DEG;
}

/** "8.0 × 10.0 × 9.0 m": width, depth and height. */
export function sizeLabel(model: ModelView): string {
  return `${model.size_m.map((metres) => metres.toFixed(1)).join(" × ")} m`;
}

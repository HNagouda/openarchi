// The sums behind moving and turning the model on the sheet. A placement is a
// point in RD New metres and a clockwise turn, as the core keeps it in
// crates/core/src/model.rs, and the footprint arrives in the model's own frame.

import type { CoveredBuilding, ModelChoice, ModelView, Placement } from "../../lib/types";
import type { Point } from "../paper";

// A building counts as replaced when the model covers more than this share of it,
// the same rule as REPLACED_SHARE in crates/core/src/model/place.rs.
export const REPLACED_SHARE = 0.5;
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

/** The buildings the model replaces by the core's rule, from what it covers. */
export function replacedBy(covered: CoveredBuilding[]): string[] {
  return covered.filter((building) => building.share > REPLACED_SHARE).map((building) => building.id);
}

/** The choice with `id` replaced or kept, as its box was ticked. */
export function withReplaced(choice: ModelChoice, id: string, isReplaced: boolean): ModelChoice {
  const others = choice.replaced.filter((entry) => entry !== id);
  return { ...choice, replaced: isReplaced ? [...others, id] : others };
}

/** "8.0 × 10.0 × 9.0 m": width, depth and height. */
export function sizeLabel(model: ModelView): string {
  return `${model.size_m.map((metres) => metres.toFixed(1)).join(" × ")} m`;
}

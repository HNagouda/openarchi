// The sums the edit tool needs between the paper and the ground: how many
// metres a paper millimetre is, and whether a ground point lies on the site.

import { isOutsideSite, paperToRd, rdToPaper, type Point } from "../paper";

/** Every view draws a tree with this crown, the radius of TREE_CROWN_RADIUS_M in crates/core/src/render/solid.rs. */
export const CROWN_RADIUS_M = 3;

/** Metres of ground per paper millimetre, or null when the sheet carries no ground points. */
export function metresPerMm(paper: Element | null): number | null {
  const toRd = paperToRd(paper);
  if (!toRd) return null;
  const origin = toRd({ x: 0, y: 0 });
  const across = toRd({ x: 1, y: 0 });
  return Math.hypot(across.x - origin.x, across.y - origin.y);
}

/** Whether an RD New point lies inside the outline the sheet draws. */
export function isOnSite(paper: Element | null, point: Point): boolean {
  const toPaper = rdToPaper(paper);
  if (!toPaper) return false;
  return !isOutsideSite(paper, toPaper(point));
}

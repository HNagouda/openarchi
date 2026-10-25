// Dev only: the View diagram's eye moved to a sheet's viewpoint in a fixture
// SVG, so placing and dragging it can be tried in a plain browser. The view
// fixtures are rendered ahead of time with the eye at the site centre, so the
// tinted ground stays the centre's whatever the viewpoint; only the eye moves.
// The real eye comes from crates/core/src/render/viewpoint.rs.

import type { Look } from "../lib/types";
import { rdToPaper } from "./mockPins";

const EYE_RADIUS_MM = 1.4;
const COLOUR = "#c4502a";
// The core's eye group holds paths and a circle but no nested group.
const CORE_EYE = /<g id="view-point"[\s\S]*?<\/g>/;

export function withMockViewPoint(svg: string, look: Look): string {
  const toPaper = rdToPaper(svg);
  const point = look.view_point;
  if (!toPaper || !point || !CORE_EYE.test(svg)) return svg;
  const [x, y] = toPaper(point[0], point[1]);
  const eye = `<g id="view-point" data-x="${x}" data-y="${y}"><circle cx="${x}" cy="${y}" r="${EYE_RADIUS_MM}" fill="#fff" stroke="${COLOUR}" stroke-width="0.4"/><circle cx="${x}" cy="${y}" r="${EYE_RADIUS_MM / 2}" fill="${COLOUR}"/></g>`;
  return svg.replace(CORE_EYE, eye);
}

// Dev only: a sheet's section line drawn into a fixture SVG, so drawing and
// dragging it can be tried in a plain browser. The plan fixtures are rendered
// ahead of time without a line, so this stands in for the core with a plain
// line and the same data attributes; the real marker comes from
// crates/core/src/render/section_marker.rs. The section view itself is one
// fixture cut along a fixed line, whatever line is drawn.

import type { Look } from "../lib/types";
import { rdToPaper } from "./mockPins";

const LINE_MM = 0.5;
const COLOUR = "#c4502a";

export function withMockSection(svg: string, look: Look): string {
  const toPaper = rdToPaper(svg);
  const line = look.section;
  if (!toPaper || !line) return svg;
  const [x1, y1] = toPaper(line.start[0], line.start[1]);
  const [x2, y2] = toPaper(line.end[0], line.end[1]);
  const group = `<g id="section-line" data-section="A" data-start-x="${x1}" data-start-y="${y1}" data-end-x="${x2}" data-end-y="${y2}"><path d="M${x1} ${y1}L${x2} ${y2}" stroke="${COLOUR}" stroke-width="${LINE_MM}" stroke-dasharray="6 1.2 0.8 1.2"/></g>`;
  return svg.replace("</svg>", `${group}</svg>`);
}

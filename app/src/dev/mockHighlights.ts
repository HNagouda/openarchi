// Dev only: highlights drawn into a fixture SVG, so drawing, moving and
// restyling them can be tried in a plain browser. The fixtures are rendered
// ahead of time without highlights, so this stands in for the core with a
// plain tinted shape and its label; the real drawing, with "dim the rest",
// comes from crates/core/src/render/highlights.rs.

import type { Area, Highlight, LonLat } from "../lib/types";
import { ringOf } from "../sheet/highlights/highlightEdits";
import { rdToPaper } from "./mockPins";

const ACCENT = "#c4502a";
const TINT_OPACITY = 0.16;
const OUTLINE_MM = 0.6;
const DASH = "1.8 1.1";
const LABEL_SIZE_MM = 3.8;
const LABEL_GAP_MM = 1.2;

const escape = (text: string) => text.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/"/g, "&quot;");

export function withMockHighlights(svg: string, highlights: Highlight[]): string {
  const toPaper = rdToPaper(svg);
  if (!toPaper || highlights.length === 0) return svg;
  const groups = highlights.map((highlight) => {
    const points = ringOf(highlight.shape).map(({ x, y }) => toPaper(x, y));
    const path = `M${points.map(([x, y]) => `${x} ${y}`).join("L")}Z`;
    const dash = highlight.outline === "dashed" ? ` stroke-dasharray="${DASH}"` : "";
    const left = Math.min(...points.map(([x]) => x));
    const right = Math.max(...points.map(([x]) => x));
    const top = Math.min(...points.map(([, y]) => y));
    const label = highlight.label
      ? `<text x="${(left + right) / 2}" y="${top - LABEL_GAP_MM}" font-size="${LABEL_SIZE_MM}" text-anchor="middle" fill="#222" stroke="#fff" stroke-width="0.6" paint-order="stroke">${escape(highlight.label)}</text>`
      : "";
    return `<path data-highlight="${highlight.id}" d="${path}" fill="${ACCENT}" fill-opacity="${TINT_OPACITY}" stroke="${ACCENT}" stroke-width="${OUTLINE_MM}"${dash}/>${label}`;
  });
  return svg.replace("</svg>", `<g id="highlights">${groups.join("")}</g></svg>`);
}

/** The sheet area a highlight opens as, as the core's `HighlightShape::to_area` gives it. */
export function mockHighlightArea(highlight: Highlight): Area {
  const shape = highlight.shape;
  if (shape.kind === "circle") return { kind: "circle", center: rdToWgs84(shape.center[0], shape.center[1]), radius_m: shape.radius_m };
  return { kind: "polygon", points: ringOf(shape).map(({ x, y }) => rdToWgs84(x, y)) };
}

// The first terms of the RD New to WGS84 series in crates/core/src/crs.rs:
// good to a metre or so around the fixture, which is all the mock needs.
const ORIGIN_X = 155_000;
const ORIGIN_Y = 463_000;
const ORIGIN_LON = 5.38720621;
const ORIGIN_LAT = 52.15517440;
const LAT_TERMS: [number, number, number][] = [[0, 1, 3235.65389], [2, 0, -32.58297], [0, 2, -0.2475], [2, 1, -0.84978], [0, 3, -0.0665], [2, 2, -0.01709]];
const LON_TERMS: [number, number, number][] = [[1, 0, 5260.52916], [1, 1, 105.94684], [1, 2, 2.45656], [3, 0, -0.81885], [1, 3, 0.05594], [3, 1, -0.05607]];

function rdToWgs84(x: number, y: number): LonLat {
  const [dx, dy] = [(x - ORIGIN_X) * 1e-5, (y - ORIGIN_Y) * 1e-5];
  const series = (terms: [number, number, number][]) => terms.reduce((sum, [p, q, c]) => sum + c * dx ** p * dy ** q, 0);
  return { lon: ORIGIN_LON + series(LON_TERMS) / 3600, lat: ORIGIN_LAT + series(LAT_TERMS) / 3600 };
}

// Dev only: pins drawn into a fixture SVG, so placing, dragging and
// relabelling pins can be tried in a plain browser. The fixtures are rendered
// ahead of time without pins, so this stands in for the core with plain
// circles at the right spots; the real markers come from
// crates/core/src/render/pins.rs.

import type { Pin } from "../lib/types";

const MARKER_RADIUS_MM = 2.4;
const LABEL_GAP_MM = 1.5;
const LABEL_SIZE_MM = 2.6;
// The core's marker scales per size, crates/core/src/pin.rs; the mock scales its circle the same.
const SIZE_SCALE = { small: 0.7, medium: 1, large: 1.45, extra_large: 2 };

/** RD New metres to paper millimetres, from the ground points on the fixture's paper rectangle. */
export function rdToPaper(svg: string): ((x: number, y: number) => [number, number]) | null {
  const values = /data-ground-points="([^"]+)"/.exec(svg)?.[1].split(" ").map(Number);
  if (!values || values.length !== 12) return null;
  // The second point lies 1 km east of the first and the third 1 km north, so each pair gives one direction.
  const [px0, py0, rx0, ry0, px1, py1, rx1, , px2, py2, , ry2] = values;
  const [ex, ey, nx, ny] = [(px1 - px0) / (rx1 - rx0), (py1 - py0) / (rx1 - rx0), (px2 - px0) / (ry2 - ry0), (py2 - py0) / (ry2 - ry0)];
  return (x, y) => [px0 + (x - rx0) * ex + (y - ry0) * nx, py0 + (x - rx0) * ey + (y - ry0) * ny];
}

const escape = (text: string) => text.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/"/g, "&quot;");

export function withMockPins(svg: string, pins: Pin[]): string {
  const toPaper = rdToPaper(svg);
  if (!toPaper || pins.length === 0) return svg;
  let numbered = 0;
  const groups = pins.map((pin) => {
    const [x, y] = toPaper(pin.x, pin.y);
    const radius = MARKER_RADIUS_MM * SIZE_SCALE[pin.size ?? "medium"];
    if (pin.icon === "number") numbered += 1;
    const mark = pin.icon === "number" ? `<text x="${x}" y="${y + 1}" font-size="${LABEL_SIZE_MM}" text-anchor="middle" fill="#fff">${numbered}</text>` : "";
    const label = pin.label ? `<text x="${x + radius + LABEL_GAP_MM}" y="${y + 1}" font-size="${LABEL_SIZE_MM}" fill="#222">${escape(pin.label)}</text>` : "";
    return `<g data-pin="${pin.id}" data-icon="${pin.icon}" data-label="${escape(pin.label)}" data-x="${x}" data-y="${y}"><circle cx="${x}" cy="${y}" r="${radius}" fill="#c4502a" stroke="#fff" stroke-width="0.4"/>${mark}${label}</g>`;
  });
  return svg.replace("</svg>", `<g id="pins">${groups.join("")}</g></svg>`);
}

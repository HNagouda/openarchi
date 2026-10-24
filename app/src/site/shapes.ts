// Changing a drawn site: switching its shape, moving it, and dragging its
// handles. Every function returns a new area and leaves the old one alone,
// so each result can go straight onto the undo stack.

import type { Area, LonLat } from "../lib/types";
import { destination, distanceM, rectangleFrom } from "./geometry";

export type Shape = Area["kind"];

// A circle dragged smaller than this is a slip, so resizing stops here.
export const MIN_RADIUS_M = 10;

// A circle becomes an octagon rather than a smooth 32-gon. The user picks a
// polygon to reshape the site by hand, and eight corners are eight handles
// they can grab; 32 would crowd the outline. A mid-edge handle adds more.
const CIRCLE_TO_POLYGON_CORNERS = 8;

// Handles in the order they are drawn: rectangle corners go south-west,
// south-east, north-east, north-west, so the opposite corner is two along.
export type Handle =
  | { role: "radius"; index: number }
  | { role: "corner"; index: number }
  | { role: "vertex"; index: number }
  | { role: "mid"; index: number };

const CARDINAL_BEARINGS = [0, 90, 180, 270];

function centreOf(points: LonLat[]): LonLat {
  const lons = points.map((point) => point.lon);
  const lats = points.map((point) => point.lat);
  return {
    lon: (Math.min(...lons) + Math.max(...lons)) / 2,
    lat: (Math.min(...lats) + Math.max(...lats)) / 2,
  };
}

function cornersOf(area: Extract<Area, { kind: "rectangle" }>): LonLat[] {
  const { south_west: sw, north_east: ne } = area;
  return [sw, { lon: ne.lon, lat: sw.lat }, ne, { lon: sw.lon, lat: ne.lat }];
}

/** The longer side of the box around some points, in metres. */
function longerSideM(points: LonLat[]): number {
  const centre = centreOf(points);
  const lons = points.map((point) => point.lon);
  const lats = points.map((point) => point.lat);
  const width = distanceM({ lon: Math.min(...lons), lat: centre.lat }, { lon: Math.max(...lons), lat: centre.lat });
  const height = distanceM({ lon: centre.lon, lat: Math.min(...lats) }, { lon: centre.lon, lat: Math.max(...lats) });
  return Math.max(width, height);
}

function squareAround(centre: LonLat, halfSideM: number): Area {
  const north = destination(centre, halfSideM, 0);
  const east = destination(centre, halfSideM, 90);
  const south = destination(centre, halfSideM, 180);
  const west = destination(centre, halfSideM, 270);
  return rectangleFrom({ lon: west.lon, lat: south.lat }, { lon: east.lon, lat: north.lat });
}

/** The points of any area as a list of corners, for converting it to a polygon or measuring it. */
function pointsOf(area: Area): LonLat[] {
  if (area.kind === "polygon") return area.points;
  if (area.kind === "rectangle") return cornersOf(area);
  return Array.from({ length: CIRCLE_TO_POLYGON_CORNERS }, (_, index) =>
    destination(area.center, area.radius_m, (360 * index) / CIRCLE_TO_POLYGON_CORNERS),
  );
}

/**
 * The same site as another shape, keeping its centre and its diameter.
 *
 * A circle becomes the square whose side is its diameter, and a rectangle or
 * polygon becomes the circle whose diameter is its longer side. Both keep the
 * number a designer reads off the site ("300 m across"), and going circle to
 * square and back returns the circle you started with.
 */
export function convertArea(area: Area, shape: Shape): Area {
  if (area.kind === shape) return area;
  if (shape === "polygon") return { kind: "polygon", points: pointsOf(area) };
  if (shape === "rectangle" && area.kind === "circle") return squareAround(area.center, area.radius_m);
  if (shape === "rectangle" && area.kind === "polygon") {
    const lons = area.points.map((point) => point.lon);
    const lats = area.points.map((point) => point.lat);
    return rectangleFrom({ lon: Math.min(...lons), lat: Math.min(...lats) }, { lon: Math.max(...lons), lat: Math.max(...lats) });
  }

  const points = pointsOf(area);
  return { kind: "circle", center: centreOf(points), radius_m: longerSideM(points) / 2 };
}

/** The area shifted by the difference between two points: dragging its body. */
export function moveArea(area: Area, from: LonLat, to: LonLat): Area {
  const shift = (point: LonLat) => ({ lon: point.lon + to.lon - from.lon, lat: point.lat + to.lat - from.lat });
  if (area.kind === "circle") return { ...area, center: shift(area.center) };
  if (area.kind === "rectangle") return { ...area, south_west: shift(area.south_west), north_east: shift(area.north_east) };
  return { ...area, points: area.points.map(shift) };
}

/** Where each handle of an area sits on the map. */
export function handlesOf(area: Area): { handle: Handle; at: LonLat }[] {
  if (area.kind === "circle") {
    return CARDINAL_BEARINGS.map((bearing, index) => ({
      handle: { role: "radius", index },
      at: destination(area.center, area.radius_m, bearing),
    }));
  }
  if (area.kind === "rectangle") {
    return cornersOf(area).map((at, index) => ({ handle: { role: "corner", index }, at }));
  }

  const vertices = area.points.map((at, index) => ({ handle: { role: "vertex", index } as Handle, at }));
  const mids = area.points.map((point, index) => {
    const next = area.points[(index + 1) % area.points.length];
    const at = { lon: (point.lon + next.lon) / 2, lat: (point.lat + next.lat) / 2 };
    return { handle: { role: "mid", index } as Handle, at };
  });
  return [...vertices, ...mids];
}

/** A polygon with a new corner after `index`, at the middle of that edge. Pressing a mid-edge handle does this. */
export function insertVertex(area: Area, index: number): Area {
  if (area.kind !== "polygon") return area;
  const point = area.points[index];
  const next = area.points[(index + 1) % area.points.length];
  const middle = { lon: (point.lon + next.lon) / 2, lat: (point.lat + next.lat) / 2 };
  return { ...area, points: [...area.points.slice(0, index + 1), middle, ...area.points.slice(index + 1)] };
}

/** The area with one handle dragged to `to`. */
export function dragHandle(area: Area, handle: Handle, to: LonLat): Area {
  if (area.kind === "circle" && handle.role === "radius") {
    return { ...area, radius_m: Math.max(MIN_RADIUS_M, distanceM(area.center, to)) };
  }
  if (area.kind === "rectangle" && handle.role === "corner") {
    const opposite = cornersOf(area)[(handle.index + 2) % 4];
    return rectangleFrom(opposite, to);
  }
  if (area.kind === "polygon" && handle.role === "vertex") {
    return { ...area, points: area.points.map((point, index) => (index === handle.index ? to : point)) };
  }
  return area;
}

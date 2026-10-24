// Map display only: drawing a drawn area on the map and measuring a radius
// while the mouse moves. The core measures and validates the real thing.

import type { Area, LonLat } from "../lib/types";

const EARTH_RADIUS_M = 6_371_008.8;
// Enough segments that a circle looks round at any zoom.
const CIRCLE_SEGMENTS = 96;

const toRadians = (degrees: number) => (degrees * Math.PI) / 180;
const toDegrees = (radians: number) => (radians * 180) / Math.PI;

/** Great-circle distance in metres. */
export function distanceM(from: LonLat, to: LonLat): number {
  const dLat = toRadians(to.lat - from.lat);
  const dLon = toRadians(to.lon - from.lon);
  const a =
    Math.sin(dLat / 2) ** 2 + Math.cos(toRadians(from.lat)) * Math.cos(toRadians(to.lat)) * Math.sin(dLon / 2) ** 2;
  return 2 * EARTH_RADIUS_M * Math.asin(Math.sqrt(a));
}

/** The point `distance` metres from `from` along `bearing` degrees. */
function destination(from: LonLat, distance: number, bearing: number): LonLat {
  const angle = distance / EARTH_RADIUS_M;
  const heading = toRadians(bearing);
  const lat1 = toRadians(from.lat);
  const lon1 = toRadians(from.lon);
  const lat2 = Math.asin(Math.sin(lat1) * Math.cos(angle) + Math.cos(lat1) * Math.sin(angle) * Math.cos(heading));
  const lon2 =
    lon1 + Math.atan2(Math.sin(heading) * Math.sin(angle) * Math.cos(lat1), Math.cos(angle) - Math.sin(lat1) * Math.sin(lat2));
  return { lon: toDegrees(lon2), lat: toDegrees(lat2) };
}

/** The outline of an area as a closed ring of [lon, lat] pairs. */
export function outline(area: Area): [number, number][] {
  if (area.kind === "circle") {
    const ring: [number, number][] = [];
    for (let i = 0; i <= CIRCLE_SEGMENTS; i++) {
      const point = destination(area.center, area.radius_m, (360 * i) / CIRCLE_SEGMENTS);
      ring.push([point.lon, point.lat]);
    }
    return ring;
  }
  if (area.kind === "rectangle") {
    const { south_west: sw, north_east: ne } = area;
    return [
      [sw.lon, sw.lat],
      [ne.lon, sw.lat],
      [ne.lon, ne.lat],
      [sw.lon, ne.lat],
      [sw.lon, sw.lat],
    ];
  }
  const ring: [number, number][] = area.points.map((point) => [point.lon, point.lat]);
  if (ring.length > 0) ring.push(ring[0]);
  return ring;
}

export function areaFeature(area: Area | null): GeoJSON.FeatureCollection {
  if (!area) return { type: "FeatureCollection", features: [] };
  return {
    type: "FeatureCollection",
    features: [{ type: "Feature", properties: {}, geometry: { type: "Polygon", coordinates: [outline(area)] } }],
  };
}

/** The polygon being drawn: placed points, plus a rubber band to the mouse. */
export function draftFeature(points: LonLat[], hover: LonLat | null): GeoJSON.FeatureCollection {
  const line = points.map((point) => [point.lon, point.lat]);
  if (hover) line.push([hover.lon, hover.lat]);
  const features: GeoJSON.Feature[] = points.map((point, index) => ({
    type: "Feature",
    properties: { isFirst: index === 0 },
    geometry: { type: "Point", coordinates: [point.lon, point.lat] },
  }));
  if (line.length >= 2) {
    features.push({ type: "Feature", properties: {}, geometry: { type: "LineString", coordinates: line } });
  }
  return { type: "FeatureCollection", features };
}

/** [[west, south], [east, north]] around an area, for fitting the map to it. */
export function bounds(area: Area): [[number, number], [number, number]] {
  const ring = outline(area);
  const lons = ring.map((point) => point[0]);
  const lats = ring.map((point) => point[1]);
  return [
    [Math.min(...lons), Math.min(...lats)],
    [Math.max(...lons), Math.max(...lats)],
  ];
}

/** Rectangle from any two opposite corners. */
export function rectangleFrom(a: LonLat, b: LonLat): Area {
  return {
    kind: "rectangle",
    south_west: { lon: Math.min(a.lon, b.lon), lat: Math.min(a.lat, b.lat) },
    north_east: { lon: Math.max(a.lon, b.lon), lat: Math.max(a.lat, b.lat) },
  };
}

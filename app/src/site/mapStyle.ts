// The basemap behind the drawing. Positron is quiet enough to draw on;
// "dark" is the same style inverted, so both themes show the same map.

import { setWorkerUrl, type Map as MapLibreMap } from "maplibre-gl";
// maplibre-gl 6 finds its worker next to its own module, which breaks once
// Vite has bundled it. Letting Vite bundle the worker too gives a URL that
// works in development and in the built app.
import mapWorkerUrl from "maplibre-gl/dist/maplibre-gl-worker.mjs?worker&url";

setWorkerUrl(mapWorkerUrl);

const LIGHT_STYLE = "https://tiles.openfreemap.org/styles/positron";
const DARK_STYLE = "https://tiles.openfreemap.org/styles/dark";

export const styleFor = (isDark: boolean) => (isDark ? DARK_STYLE : LIGHT_STYLE);

// The Netherlands, for a new site with nothing searched yet.
export const NETHERLANDS_CENTER: [number, number] = [5.29, 52.13];
export const NETHERLANDS_ZOOM = 7;
// Close enough to see buildings when jumping to an address.
export const ADDRESS_ZOOM = 16.5;
export const PLACE_ZOOM = 13;

export const SITE_SOURCE = "site";
export const DRAFT_SOURCE = "draft";
export const HANDLE_SOURCE = "handles";
export const SITE_FILL_LAYER = "site-fill";
// Corners draw above the mid-edge handles, so a corner wins where the two overlap on a short edge.
export const HANDLE_LAYERS = ["handles-mid", "handles"];

/** Adds the drawn site and the polygon draft on top of the basemap. Safe to call after every style load. */
export function addSiteLayers(map: MapLibreMap, brand: string) {
  if (map.getSource(SITE_SOURCE)) return;
  const empty: GeoJSON.FeatureCollection = { type: "FeatureCollection", features: [] };
  map.addSource(SITE_SOURCE, { type: "geojson", data: empty });
  map.addSource(DRAFT_SOURCE, { type: "geojson", data: empty });
  map.addSource(HANDLE_SOURCE, { type: "geojson", data: empty });

  map.addLayer({ id: SITE_FILL_LAYER, type: "fill", source: SITE_SOURCE, paint: { "fill-color": brand, "fill-opacity": 0.09 } });
  map.addLayer({
    id: "site-line",
    type: "line",
    source: SITE_SOURCE,
    paint: { "line-color": brand, "line-width": 2 },
    layout: { "line-join": "round" },
  });
  map.addLayer({
    id: "draft-line",
    type: "line",
    source: DRAFT_SOURCE,
    filter: ["==", ["geometry-type"], "LineString"],
    paint: { "line-color": brand, "line-width": 1.5, "line-dasharray": [2, 1.5] },
  });
  map.addLayer({
    id: "draft-points",
    type: "circle",
    source: DRAFT_SOURCE,
    filter: ["==", ["geometry-type"], "Point"],
    paint: {
      "circle-radius": ["case", ["get", "isFirst"], 5, 3.5],
      "circle-color": "#ffffff",
      "circle-stroke-color": brand,
      "circle-stroke-width": 1.5,
    },
  });
  map.addLayer({
    id: "handles-mid",
    type: "circle",
    source: HANDLE_SOURCE,
    filter: ["==", ["get", "role"], "mid"],
    paint: { "circle-radius": 3.5, "circle-color": brand, "circle-opacity": 0.55, "circle-stroke-color": "#ffffff", "circle-stroke-width": 1 },
  });
  map.addLayer({
    id: "handles",
    type: "circle",
    source: HANDLE_SOURCE,
    filter: ["!=", ["get", "role"], "mid"],
    paint: { "circle-radius": 5, "circle-color": "#ffffff", "circle-stroke-color": brand, "circle-stroke-width": 1.5 },
  });
}

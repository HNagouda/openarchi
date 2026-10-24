// Editing a drawn site with the pan tool: click it to select it, then drag its
// body to move it, drag a handle to resize it, or press a polygon's mid-edge
// handle to add a corner there. A click on the empty map deselects.
//
// Only a selected site moves under a drag. A site often fills the window, and
// dragging inside an unselected one must still pan the map. Every drag goes
// to `onChange` under one merge key and is sealed when the mouse lifts, so a
// whole drag undoes in one step.

import { useEffect, useRef } from "react";
import type { Map as MapLibreMap, MapMouseEvent, PointLike } from "maplibre-gl";
import type { Area, LonLat } from "../lib/types";
import { HANDLE_LAYERS, HANDLE_SOURCE, SITE_FILL_LAYER } from "./mapStyle";
import { dragHandle, handlesOf, insertVertex, moveArea, type Handle } from "./shapes";
import type { Tool } from "./useDrawing";

// Handles are small; this much slack around the pointer still grabs one.
const HANDLE_HIT_PX = 6;
const EDIT_MERGE_KEY = "edit";

type Gesture = { start: Area; from: LonLat; handle: Handle | null };

type Options = {
  map: MapLibreMap | null;
  tool: Tool;
  area: Area | null;
  isSelected: boolean;
  onSelect: (isSelected: boolean) => void;
  onChange: (area: Area, key: string) => void;
  onSeal: () => void;
};

const lonLatOf = (event: MapMouseEvent): LonLat => ({ lon: event.lngLat.lng, lat: event.lngLat.lat });

function handleFeatures(area: Area | null): GeoJSON.FeatureCollection {
  if (!area) return { type: "FeatureCollection", features: [] };
  return {
    type: "FeatureCollection",
    features: handlesOf(area).map(({ handle, at }) => ({
      type: "Feature",
      properties: { role: handle.role, index: handle.index },
      geometry: { type: "Point", coordinates: [at.lon, at.lat] },
    })),
  };
}

/** The resize cursor that points the way a handle moves. */
function cursorFor(handle: Handle): string {
  if (handle.role === "radius") return handle.index % 2 === 0 ? "ns-resize" : "ew-resize";
  // Corners go south-west, south-east, north-east, north-west.
  if (handle.role === "corner") return handle.index % 2 === 0 ? "nesw-resize" : "nwse-resize";
  if (handle.role === "mid") return "copy";
  return "pointer";
}

export function useEditing({ map, tool, area, isSelected, onSelect, onChange, onSeal }: Options) {
  const gesture = useRef<Gesture | null>(null);
  const latest = useRef({ tool, area, isSelected, onSelect, onChange, onSeal });
  latest.current = { tool, area, isSelected, onSelect, onChange, onSeal };

  // Handles show only on a selected site, and only while no drawing tool is out.
  useEffect(() => {
    if (!map) return;
    const shown = isSelected && tool === "pan" ? area : null;
    const draw = () => {
      const source = map.getSource(HANDLE_SOURCE) as { setData?: (data: GeoJSON.FeatureCollection) => void } | undefined;
      source?.setData?.(handleFeatures(shown));
    };
    draw();
    map.on("styledata", draw);
    return () => {
      map.off("styledata", draw);
    };
  }, [map, area, isSelected, tool]);

  useEffect(() => {
    if (!map) return;

    const handleAt = (event: MapMouseEvent): Handle | null => {
      if (!latest.current.isSelected) return null;
      const { x, y } = event.point;
      const box: [PointLike, PointLike] = [
        [x - HANDLE_HIT_PX, y - HANDLE_HIT_PX],
        [x + HANDLE_HIT_PX, y + HANDLE_HIT_PX],
      ];
      const [feature] = map.queryRenderedFeatures(box, { layers: HANDLE_LAYERS });
      if (!feature) return null;
      return { role: feature.properties.role, index: Number(feature.properties.index) } as Handle;
    };

    const isOnBody = (event: MapMouseEvent) => map.queryRenderedFeatures(event.point, { layers: [SITE_FILL_LAYER] }).length > 0;

    const hoverCursor = (event: MapMouseEvent) => {
      if (!latest.current.area) return "";
      const handle = handleAt(event);
      if (handle) return cursorFor(handle);
      if (!isOnBody(event)) return "";
      return latest.current.isSelected ? "move" : "pointer";
    };

    const onDown = (event: MapMouseEvent) => {
      const { tool: current, area: drawn, isSelected: isPicked } = latest.current;
      if (current !== "pan" || !drawn || !isPicked) return;
      const handle = handleAt(event);
      if (!handle && !isOnBody(event)) return;
      // Stops the map from panning while the site is dragged.
      event.preventDefault();
      latest.current.onSeal();

      if (handle?.role === "mid") {
        // Pressing a mid-edge handle adds a corner there and drags it, in one undo step.
        const added = insertVertex(drawn, handle.index);
        latest.current.onChange(added, EDIT_MERGE_KEY);
        gesture.current = { start: added, from: lonLatOf(event), handle: { role: "vertex", index: handle.index + 1 } };
      } else {
        gesture.current = { start: drawn, from: lonLatOf(event), handle };
      }

      const end = () => {
        gesture.current = null;
        latest.current.onSeal();
        window.removeEventListener("mouseup", end);
      };
      window.addEventListener("mouseup", end);
    };

    const onMove = (event: MapMouseEvent) => {
      const active = gesture.current;
      if (active) {
        const here = lonLatOf(event);
        const next = active.handle ? dragHandle(active.start, active.handle, here) : moveArea(active.start, active.from, here);
        latest.current.onChange(next, EDIT_MERGE_KEY);
        return;
      }
      if (latest.current.tool !== "pan") return;
      map.getCanvas().style.cursor = hoverCursor(event);
    };

    const onClick = (event: MapMouseEvent) => {
      if (latest.current.tool !== "pan" || !latest.current.area) return;
      latest.current.onSelect(handleAt(event) !== null || isOnBody(event));
    };

    map.on("mousedown", onDown);
    map.on("mousemove", onMove);
    map.on("click", onClick);
    return () => {
      map.off("mousedown", onDown);
      map.off("mousemove", onMove);
      map.off("click", onClick);
    };
  }, [map]);
}

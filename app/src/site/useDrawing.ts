// Drawing a site on the map: drag out a circle or rectangle, or click the
// corners of a polygon. The finished shape goes to `onArea`; while drawing,
// `live` carries a small label for the cursor (the radius, the size).

import { useCallback, useEffect, useRef, useState } from "react";
import type { Map as MapLibreMap, MapMouseEvent } from "maplibre-gl";
import { formatDistance } from "../lib/format";
import type { Area, LonLat } from "../lib/types";
import { DRAFT_SOURCE, SITE_SOURCE } from "./mapStyle";
import { areaFeature, distanceM, draftFeature, rectangleFrom } from "./geometry";

export type Tool = "pan" | "circle" | "rectangle" | "polygon";

// Smaller than this is a slip of the mouse, not a site.
const MIN_RADIUS_M = 10;
// A click this close to the first corner closes the polygon.
const CLOSE_DISTANCE_PX = 10;
const SAME_SPOT_PX = 4;

type LiveLabel = { x: number; y: number; text: string };

type Options = {
  map: MapLibreMap | null;
  tool: Tool;
  area: Area | null;
  onArea: (area: Area) => void;
  onDone: () => void;
};

const lonLatOf = (event: MapMouseEvent): LonLat => ({ lon: event.lngLat.lng, lat: event.lngLat.lat });

function setSourceData(map: MapLibreMap, id: string, data: GeoJSON.FeatureCollection) {
  const source = map.getSource(id) as { setData?: (data: GeoJSON.FeatureCollection) => void } | undefined;
  source?.setData?.(data);
}

export function useDrawing({ map, tool, area, onArea, onDone }: Options) {
  const [points, setPoints] = useState<LonLat[]>([]);
  const [hover, setHover] = useState<LonLat | null>(null);
  const [live, setLive] = useState<LiveLabel | null>(null);
  const dragStart = useRef<LonLat | null>(null);

  const latest = useRef({ tool, onArea, onDone, points });
  latest.current = { tool, onArea, onDone, points };

  // Keep the map's site layer in step with the area, also after a style swap.
  useEffect(() => {
    if (!map) return;
    const draw = () => setSourceData(map, SITE_SOURCE, areaFeature(area));
    draw();
    map.on("styledata", draw);
    return () => {
      map.off("styledata", draw);
    };
  }, [map, area]);

  useEffect(() => {
    if (!map) return;
    setSourceData(map, DRAFT_SOURCE, draftFeature(points, tool === "polygon" ? hover : null));
  }, [map, points, hover, tool]);

  // Dragging draws a circle or rectangle, so the map must not pan meanwhile.
  useEffect(() => {
    if (!map) return;
    const isDragTool = tool === "circle" || tool === "rectangle";
    if (isDragTool) map.dragPan.disable();
    else map.dragPan.enable();
    if (tool === "polygon") map.doubleClickZoom.disable();
    else map.doubleClickZoom.enable();
    map.getCanvas().style.cursor = tool === "pan" ? "" : "crosshair";
    setPoints([]);
    setHover(null);
    setLive(null);
  }, [map, tool]);

  const finishPolygon = useCallback(() => {
    const placed = latest.current.points;
    if (placed.length < 3) return;
    latest.current.onArea({ kind: "polygon", points: placed });
    setPoints([]);
    setHover(null);
    latest.current.onDone();
  }, []);

  const removeLastPoint = useCallback(() => setPoints((current) => current.slice(0, -1)), []);

  const cancel = useCallback(() => {
    dragStart.current = null;
    setPoints([]);
    setHover(null);
    setLive(null);
  }, []);

  useEffect(() => {
    if (!map) return;

    const onDown = (event: MapMouseEvent) => {
      const current = latest.current.tool;
      if (current !== "circle" && current !== "rectangle") return;
      event.preventDefault();
      dragStart.current = lonLatOf(event);
    };

    const onMove = (event: MapMouseEvent) => {
      const current = latest.current.tool;
      const here = lonLatOf(event);
      const start = dragStart.current;
      if (current === "polygon") {
        setHover(here);
        return;
      }
      if (!start) return;

      if (current === "circle") {
        const radius = distanceM(start, here);
        latest.current.onArea({ kind: "circle", center: start, radius_m: radius });
        setLive({ x: event.point.x, y: event.point.y, text: formatDistance(radius) });
      }
      if (current === "rectangle") {
        latest.current.onArea(rectangleFrom(start, here));
        const width = distanceM(start, { lon: here.lon, lat: start.lat });
        const height = distanceM(start, { lon: start.lon, lat: here.lat });
        setLive({ x: event.point.x, y: event.point.y, text: `${formatDistance(width)} × ${formatDistance(height)}` });
      }
    };

    const onUp = (event: MapMouseEvent) => {
      const start = dragStart.current;
      if (!start) return;
      dragStart.current = null;
      setLive(null);
      if (distanceM(start, lonLatOf(event)) < MIN_RADIUS_M) return;
      latest.current.onDone();
    };

    const onClick = (event: MapMouseEvent) => {
      if (latest.current.tool !== "polygon") return;
      const placed = latest.current.points;
      if (placed.length >= 3) {
        const first = map.project([placed[0].lon, placed[0].lat]);
        if (first.dist(event.point) < CLOSE_DISTANCE_PX) {
          finishPolygon();
          return;
        }
      }
      // The two clicks of a double-click land on the same spot; keep one corner.
      const last = placed[placed.length - 1];
      if (last && map.project([last.lon, last.lat]).dist(event.point) < SAME_SPOT_PX) return;
      setPoints([...placed, lonLatOf(event)]);
    };

    const onDoubleClick = (event: MapMouseEvent) => {
      if (latest.current.tool !== "polygon") return;
      event.preventDefault();
      finishPolygon();
    };

    map.on("mousedown", onDown);
    map.on("mousemove", onMove);
    map.on("mouseup", onUp);
    map.on("click", onClick);
    map.on("dblclick", onDoubleClick);
    return () => {
      map.off("mousedown", onDown);
      map.off("mousemove", onMove);
      map.off("mouseup", onUp);
      map.off("click", onClick);
      map.off("dblclick", onDoubleClick);
    };
  }, [map, finishPolygon]);

  return { live, pointCount: points.length, finishPolygon, removeLastPoint, cancel };
}

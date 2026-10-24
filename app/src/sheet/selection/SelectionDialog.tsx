// Drawing a sheet's sub-selection: a smaller area inside the fetched site that
// the sheet frames and zooms to.
//
// It is drawn on the same map, with the same circle, rectangle and polygon
// tools and the same handles, as the site itself, because a sub-selection is
// an area of exactly that kind. The fetched site shows as a dashed outline to
// draw inside; whatever reaches past it is cut off when the sheet is drawn.

import { useEffect, useRef, useState } from "react";
import { AttributionControl, Map as MapLibreMap } from "maplibre-gl";
import { useIsDark } from "../../lib/appearance";
import { isTyping, useKeyDown } from "../../lib/keys";
import type { Area } from "../../lib/types";
import { Button } from "../../ui/Button";
import { Dialog } from "../../ui/Dialog";
import { areaFeature, bounds } from "../../site/geometry";
import { addSiteLayers, styleFor } from "../../site/mapStyle";
import { useDrawing, type Tool } from "../../site/useDrawing";
import { useEditing } from "../../site/useEditing";
import "./selection.css";

const DIALOG_WIDTH_PX = 760;
const FIT_PADDING_PX = 40;
const WHOLE_SITE_SOURCE = "whole-site";
const WHOLE_SITE_LAYER = "whole-site-line";
// The fetched site in a quiet dashed line, under the brand-coloured selection.
const WHOLE_SITE_COLOUR = "#7a7468";

const TOOLS: { tool: Tool; icon: "circle" | "rectangle" | "polygon"; tip: string; key: string }[] = [
  { tool: "circle", icon: "circle", tip: "Circle", key: "c" },
  { tool: "rectangle", icon: "rectangle", tip: "Rectangle", key: "r" },
  { tool: "polygon", icon: "polygon", tip: "Polygon", key: "p" },
];

type Props = {
  /** The fetched site. */
  site: Area;
  selection: Area | null;
  onApply: (selection: Area | null) => void;
  onClose: () => void;
};

/** The hint under the map for what the mouse does right now. */
function statusFor(tool: Tool, pointCount: number, selection: Area | null): string {
  if (tool === "polygon" && pointCount < 3) return "Click each corner of the area";
  if (tool === "polygon") return "Click the first corner or press Return to finish";
  if (tool === "circle") return "Drag from the centre outwards";
  if (tool === "rectangle") return "Drag from one corner to the opposite one";
  if (selection) return "Click the area to move or reshape it, or pick a tool to draw a new one";
  return "Pick a tool and draw the area this sheet should show";
}

function brandColour() {
  return getComputedStyle(document.documentElement).getPropertyValue("--brand").trim() || "#e0572a";
}

function addWholeSite(map: MapLibreMap, site: Area) {
  if (map.getSource(WHOLE_SITE_SOURCE)) return;
  map.addSource(WHOLE_SITE_SOURCE, { type: "geojson", data: areaFeature(site) });
  map.addLayer({
    id: WHOLE_SITE_LAYER,
    type: "line",
    source: WHOLE_SITE_SOURCE,
    paint: { "line-color": WHOLE_SITE_COLOUR, "line-width": 1.5, "line-dasharray": [3, 2] },
  });
}

export function SelectionDialog({ site, selection, onApply, onClose }: Props) {
  const container = useRef<HTMLDivElement>(null);
  const [map, setMap] = useState<MapLibreMap | null>(null);
  const [area, setArea] = useState<Area | null>(selection);
  const [tool, setTool] = useState<Tool>(selection ? "pan" : "rectangle");
  const [isPicked, setPicked] = useState(false);
  const isDark = useIsDark();

  // The map lives as long as the dialog; the site outline goes under the drawing layers.
  useEffect(() => {
    if (!container.current) return;
    const created = new MapLibreMap({
      container: container.current,
      style: styleFor(isDark),
      bounds: bounds(site),
      fitBoundsOptions: { padding: FIT_PADDING_PX },
      attributionControl: false,
      pitchWithRotate: false,
      dragRotate: false,
    });
    created.addControl(new AttributionControl({ compact: true }), "bottom-left");
    created.on("style.load", () => {
      addWholeSite(created, site);
      addSiteLayers(created, brandColour());
    });
    setMap(created);
    return () => created.remove();
    // Created once, for the site the dialog opened with.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const drawing = useDrawing({
    map,
    tool,
    area,
    onArea: setArea,
    onDone: () => {
      setPicked(true);
      setTool("pan");
    },
  });

  useEditing({ map, tool, area, isSelected: isPicked && area !== null, onSelect: setPicked, onChange: setArea, onSeal: () => undefined });

  useKeyDown((event) => {
    if (isTyping(event.target) || event.metaKey || event.ctrlKey || event.altKey) return;
    const lower = event.key.toLowerCase();
    const picked = TOOLS.find((entry) => entry.key === lower);
    if (picked) setTool(picked.tool);
    if (event.key === "Enter" && tool === "polygon") drawing.finishPolygon();
    if (event.key === "Enter" && tool === "pan") onApply(area);
    const isDeleteKey = event.key === "Backspace" || event.key === "Delete";
    if (isDeleteKey && tool === "polygon") drawing.removeLastPoint();
    if (isDeleteKey && tool === "pan" && !event.repeat) setArea(null);
  });

  const footer = (
    <>
      <Button variant="ghost" disabled={area === null} onClick={() => setArea(null)}>
        Whole site
      </Button>
      <span className="dialog-footer-spacer" />
      <Button variant="secondary" onClick={onClose}>
        Cancel
      </Button>
      <Button variant="primary" onClick={() => onApply(area)}>
        {area ? "Show this area" : "Show the whole site"}
      </Button>
    </>
  );

  return (
    <Dialog title="Sheet area" onClose={onClose} footer={footer} width={DIALOG_WIDTH_PX}>
      <p className="selection-intro">Draw a smaller area inside the site. This sheet zooms to it, and its ratios describe only that area.</p>
      <div className="selection-map-wrap">
        <div ref={container} className="selection-map" />
        <div className="selection-tools" role="toolbar" aria-label="Drawing tools">
          {TOOLS.map((entry) => (
            <Button
              key={entry.tool}
              icon={entry.icon}
              tip={entry.tip}
              shortcut={entry.key}
              isActive={tool === entry.tool}
              onClick={() => setTool(tool === entry.tool ? "pan" : entry.tool)}
            />
          ))}
        </div>
        <div className="selection-status">{statusFor(tool, drawing.pointCount, area)}</div>
      </div>
    </Dialog>
  );
}

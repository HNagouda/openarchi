// The site step: a map to find the place, tools to draw it, and the fetch
// that turns the drawing into a scene.

import { useEffect, useRef, useState } from "react";
import { AttributionControl, Map as MapLibreMap, NavigationControl } from "maplibre-gl";
import type { Navigate } from "../App";
import { useIsDark } from "../lib/appearance";
import { useCommandHandler } from "../lib/commands";
import {
  cancelFetch,
  closeProject,
  createProject,
  currentProject,
  errorText,
  fetchSite,
  type FetchFinishedEvent,
  type FetchProgressEvent,
  measureArea,
  onFetchFinished,
  onFetchProgress,
  renameProject,
} from "../lib/ipc";
import { usePlainKeys } from "../lib/keys";
import type { Area, AreaSize, Place, ProjectView } from "../lib/types";
import { Button } from "../ui/Button";
import { useToast } from "../ui/Toasts";
import { bounds } from "./geometry";
import { ADDRESS_ZOOM, NETHERLANDS_CENTER, NETHERLANDS_ZOOM, PLACE_ZOOM, addSiteLayers, styleFor } from "./mapStyle";
import { SearchBox } from "./SearchBox";
import { SitePanel, type FetchState } from "./SitePanel";
import { useDrawing, type Tool } from "./useDrawing";

// Kinds the geocoder uses for a single address, which deserve a close zoom.
const ADDRESS_KINDS = ["adres", "address", "postcode"];
const FIT_PADDING_PX = 80;
const NO_FETCH = -1;
const STARTING = -2;

type Props = { project: ProjectView | null; navigate: Navigate };

const TOOLS: { tool: Tool; icon: "back" | "circle" | "rectangle" | "polygon"; tip: string; key: string }[] = [
  { tool: "circle", icon: "circle", tip: "Circle", key: "c" },
  { tool: "rectangle", icon: "rectangle", tip: "Rectangle", key: "r" },
  { tool: "polygon", icon: "polygon", tip: "Polygon", key: "p" },
];

function brandColour() {
  return getComputedStyle(document.documentElement).getPropertyValue("--brand").trim() || "#e0572a";
}

export function SiteStep({ project, navigate }: Props) {
  const container = useRef<HTMLDivElement>(null);
  const [map, setMap] = useState<MapLibreMap | null>(null);
  const [tool, setTool] = useState<Tool>(project ? "pan" : "circle");
  const [area, setArea] = useState<Area | null>(project?.area ?? null);
  const [size, setSize] = useState<AreaSize | null>(null);
  const [name, setName] = useState(project?.name ?? "");
  const [fetch, setFetch] = useState<FetchState>({ state: "idle" });
  const [createdProject, setCreatedProject] = useState<ProjectView | null>(project);
  // Which fetch this step listens to. While the command is still returning its
  // number, early events wait in `early` so a fast failure is not lost.
  const generation = useRef(NO_FETCH);
  const early = useRef<{ progress: FetchProgressEvent[]; finished: FetchFinishedEvent[] }>({ progress: [], finished: [] });
  const isDark = useIsDark();
  const toast = useToast();

  // The map lives for as long as the step does; its style follows the appearance.
  useEffect(() => {
    if (!container.current) return;
    const created = new MapLibreMap({
      container: container.current,
      style: styleFor(isDark),
      center: NETHERLANDS_CENTER,
      zoom: NETHERLANDS_ZOOM,
      attributionControl: false,
      pitchWithRotate: false,
      dragRotate: false,
    });
    created.addControl(new AttributionControl({ compact: true }), "bottom-left");
    created.addControl(new NavigationControl({ showCompass: false }), "bottom-left");
    created.on("style.load", () => addSiteLayers(created, brandColour()));
    if (project) created.fitBounds(bounds(project.area), { padding: FIT_PADDING_PX, duration: 0 });
    setMap(created);
    return () => created.remove();
    // Created once; the appearance effect below swaps the style.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    map?.setStyle(styleFor(isDark));
  }, [map, isDark]);

  useEffect(() => {
    if (!area) {
      setSize(null);
      return;
    }
    let isCurrent = true;
    measureArea(area)
      .then((measured) => isCurrent && setSize(measured))
      .catch(() => undefined);
    return () => {
      isCurrent = false;
    };
  }, [area]);

  const drawing = useDrawing({ map, tool, area, onArea: setArea, onDone: () => setTool("pan") });

  const handleProgress = (event: FetchProgressEvent) => {
    if (generation.current === STARTING) early.current.progress.push(event);
    if (event.generation !== generation.current) return;
    setFetch({ state: "running", progress: event.progress });
  };

  const handleFinished = async (event: FetchFinishedEvent) => {
    if (generation.current === STARTING) early.current.finished.push(event);
    if (event.generation !== generation.current) return;
    generation.current = NO_FETCH;
    if (event.error) {
      setFetch({ state: "failed", error: event.error });
      return;
    }
    try {
      const fetched = await currentProject();
      navigate.sheet(fetched);
      // A failed source still leaves a usable site, so say what is missing rather than block.
      if (fetched.warnings.length > 0) {
        toast(`Opened without some data: ${fetched.warnings.join("; ")}`, { isError: true });
      }
    } catch (error) {
      setFetch({ state: "failed", error: errorText(error) });
    }
  };

  const handlers = useRef({ handleProgress, handleFinished });
  handlers.current = { handleProgress, handleFinished };

  // Progress and the result of a fetch arrive as events from the Rust side.
  useEffect(() => {
    const stopProgress = onFetchProgress((event) => handlers.current.handleProgress(event));
    const stopFinished = onFetchFinished((event) => handlers.current.handleFinished(event));
    return () => {
      stopProgress.then((stop) => stop());
      stopFinished.then((stop) => stop());
    };
  }, []);

  const pickPlace = (place: Place) => {
    if (!name) setName(place.label.split(",")[0]);
    const zoom = ADDRESS_KINDS.includes(place.kind) ? ADDRESS_ZOOM : PLACE_ZOOM;
    map?.flyTo({ center: [place.location.lon, place.location.lat], zoom, duration: 900, essential: true });
  };

  const startFetch = async () => {
    if (!area) return;
    setFetch({ state: "running", progress: null });
    try {
      let target = createdProject;
      if (!target) {
        target = await createProject(name || "Untitled site", area);
        setCreatedProject(target);
      }
      early.current = { progress: [], finished: [] };
      generation.current = STARTING;
      const started = await fetchSite(area);
      generation.current = started;
      const { progress, finished } = early.current;
      progress.filter((event) => event.generation === started).forEach(handleProgress);
      finished.filter((event) => event.generation === started).forEach(handleFinished);
    } catch (error) {
      generation.current = NO_FETCH;
      setFetch({ state: "failed", error: errorText(error) });
    }
  };

  const stopFetch = () => {
    generation.current = NO_FETCH;
    cancelFetch();
    setFetch({ state: "idle" });
  };

  const rename = (next: string) => {
    setName(next);
    if (createdProject) renameProject(createdProject.id, next).catch(() => undefined);
  };

  const leave = () => {
    if (fetch.state === "running") stopFetch();
    if (project?.has_scene) {
      navigate.sheet(project);
      return;
    }
    closeProject().catch(() => undefined);
    navigate.home();
  };

  const setRadius = (radius: number) => {
    if (area?.kind === "circle") setArea({ ...area, radius_m: radius });
  };

  useCommandHandler((id) => {
    if (id === "home") {
      leave();
      return true;
    }
    return false;
  });

  usePlainKeys((key, event) => {
    const lower = key.toLowerCase();
    if (lower === "c") setTool("circle");
    if (lower === "r") setTool("rectangle");
    if (lower === "p") setTool("polygon");
    if (lower === "h" || lower === "v") setTool("pan");
    if (key === "Enter" && tool === "polygon") drawing.finishPolygon();
    if (key === "Backspace" && tool === "polygon") {
      drawing.removeLastPoint();
      event.preventDefault();
    }
    if (key === "Escape") {
      drawing.cancel();
      setTool("pan");
    }
  });

  const statusText =
    tool === "polygon"
      ? drawing.pointCount < 3
        ? "Click each corner of the site"
        : "Click the first corner or press Return to finish"
      : tool === "circle"
        ? "Drag from the centre outwards"
        : tool === "rectangle"
          ? "Drag from one corner to the opposite one"
          : null;

  return (
    <div className="site-step">
      <header className="topbar" data-tauri-drag-region>
        <Button icon="back" tip={project?.has_scene ? "Back to the sheet" : "All projects"} onClick={leave} />
        <div className="topbar-title" data-tauri-drag-region>
          {project ? "Edit site" : "New site"}
        </div>
        <div className="topbar-spacer" data-tauri-drag-region />
        <span className="topbar-subtle">Netherlands only for now</span>
      </header>

      <div className="site-body">
        <div className="map-wrap">
          <div ref={container} className="map" />
          <div className="map-overlay-top">
            <SearchBox onPick={pickPlace} />
            <div className="tool-palette" role="toolbar" aria-label="Drawing tools">
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
          </div>
          {statusText && <div className="draw-status">{statusText}</div>}
          {drawing.live && (
            <div className="live-label tabular" style={{ left: drawing.live.x + 14, top: drawing.live.y + 14 }}>
              {drawing.live.text}
            </div>
          )}
        </div>

        <SitePanel
          name={name}
          onName={rename}
          area={area}
          size={size}
          onRadius={setRadius}
          fetch={fetch}
          onFetch={() => {
            startFetch().catch((error) => toast(errorText(error), { isError: true }));
          }}
          onCancel={stopFetch}
          isEditing={project !== null}
        />
      </div>
    </div>
  );
}

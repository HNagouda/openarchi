// The site step: a map to find the place, tools to draw it, and the fetch
// that turns the drawing into a scene.

import { useEffect, useRef, useState } from "react";
import { AttributionControl, Map as MapLibreMap, ScaleControl } from "maplibre-gl";
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
import { useHistory } from "../lib/history";
import { isTyping, usePlainKeys } from "../lib/keys";
import type { Area, AreaSize, Place, ProjectView } from "../lib/types";
import { Button } from "../ui/Button";
import { useToast } from "../ui/Toasts";
import { ZoomControl } from "../ui/ZoomControl";
import { bounds } from "./geometry";
import { ADDRESS_ZOOM, NETHERLANDS_CENTER, NETHERLANDS_ZOOM, PLACE_ZOOM, addSiteLayers, styleFor } from "./mapStyle";
import { CIRCLE_KEY, PAN_KEYS, POLYGON_KEY, RECTANGLE_KEY, SitePanel, type FetchState } from "./SitePanel";
import { convertArea, type Shape } from "./shapes";
import { useDrawing, type Tool } from "./useDrawing";
import { useEditing } from "./useEditing";

// Kinds the geocoder uses for a single address, which deserve a close zoom.
const ADDRESS_KINDS = ["adres", "address", "postcode"];
const FIT_PADDING_PX = 80;
const RESET_VIEW_MS = 600;
// The scale bar picks a round length that fits in this many pixels.
const SCALE_BAR_MAX_PX = 120;
// Each drag or slider sends many changes under one of these keys; the history
// folds them into one undo step until the gesture is sealed.
const DRAW_MERGE_KEY = "draw";
const RADIUS_MERGE_KEY = "radius";
const NO_FETCH = -1;
const STARTING = -2;

type Props = { project: ProjectView | null; navigate: Navigate };

/** The hint at the bottom of the map for what the mouse does right now. */
function statusFor(tool: Tool, pointCount: number, selected: Area | null): string | null {
  if (tool === "polygon" && pointCount < 3) return "Click each corner of the site";
  if (tool === "polygon") return "Click the first corner or press Return to finish";
  if (tool === "circle") return "Drag from the centre outwards";
  if (tool === "rectangle") return "Drag from one corner to the opposite one";
  if (selected?.kind === "polygon") return "Drag a corner to move it, or a dot between corners to add one";
  if (selected) return "Drag to move, or drag a handle to resize";
  return null;
}

function brandColour() {
  return getComputedStyle(document.documentElement).getPropertyValue("--brand").trim() || "#e0572a";
}

export function SiteStep({ project, navigate }: Props) {
  const container = useRef<HTMLDivElement>(null);
  const [map, setMap] = useState<MapLibreMap | null>(null);
  const [tool, setTool] = useState<Tool>(project ? "pan" : "circle");
  const history = useHistory<Area | null>(project?.area ?? null);
  const area = history.present;
  const [isPicked, setPicked] = useState(false);
  const isSelected = isPicked && area !== null;
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
    // Added after the credits, so MapLibre stacks it above them in the same corner.
    created.addControl(new ScaleControl({ unit: "metric", maxWidth: SCALE_BAR_MAX_PX }), "bottom-left");
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

  const drawing = useDrawing({
    map,
    tool,
    area,
    onArea: (drawn) => history.change(drawn, DRAW_MERGE_KEY),
    onDone: () => {
      history.seal();
      setPicked(true);
      setTool("pan");
    },
  });

  useEditing({ map, tool, area, isSelected, onSelect: setPicked, onChange: history.change, onSeal: history.seal });

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
    if (area?.kind === "circle") history.change({ ...area, radius_m: radius }, RADIUS_MERGE_KEY);
  };

  const changeShape = (shape: Shape) => {
    if (area) history.change(convertArea(area, shape));
  };

  const deleteArea = () => {
    history.change(null);
    setPicked(false);
  };

  // Back to the drawn site, or to the whole country when nothing is drawn yet.
  const resetView = () => {
    if (!map) return;
    if (area) {
      map.fitBounds(bounds(area), { padding: FIT_PADDING_PX, duration: RESET_VIEW_MS, bearing: 0, pitch: 0 });
      return;
    }
    map.flyTo({ center: NETHERLANDS_CENTER, zoom: NETHERLANDS_ZOOM, duration: RESET_VIEW_MS, bearing: 0, pitch: 0 });
  };

  useCommandHandler((id) => {
    // In a text field, undo belongs to the typing, so the window passes it on.
    if ((id === "undo" || id === "redo") && isTyping(document.activeElement)) return false;
    if (id === "undo") history.undo();
    if (id === "redo") history.redo();
    if (id === "zoom-in") map?.zoomIn();
    if (id === "zoom-out") map?.zoomOut();
    if (id === "zoom-fit") resetView();
    if (id === "home") leave();
    return ["undo", "redo", "zoom-in", "zoom-out", "zoom-fit", "home"].includes(id);
  });

  usePlainKeys((key, event) => {
    const lower = key.toLowerCase();
    if (lower === CIRCLE_KEY) setTool("circle");
    if (lower === RECTANGLE_KEY) setTool("rectangle");
    if (lower === POLYGON_KEY) setTool("polygon");
    if (PAN_KEYS.includes(lower)) setTool("pan");
    if (key === "Enter" && tool === "polygon") drawing.finishPolygon();
    const isDeleteKey = key === "Backspace" || key === "Delete";
    if (isDeleteKey && tool === "polygon") {
      drawing.removeLastPoint();
      event.preventDefault();
    }
    // Holding the key down must not delete the next thing too.
    if (isDeleteKey && tool === "pan" && isSelected && !event.repeat) {
      deleteArea();
      event.preventDefault();
    }
    if (key === "Escape") {
      drawing.cancel();
      setTool("pan");
      setPicked(false);
    }
  });

  const statusText = statusFor(tool, drawing.pointCount, isSelected ? area : null);

  return (
    <div className="site-step">
      <header className="topbar" data-tauri-drag-region>
        <Button icon="back" tip={project?.has_scene ? "Back to the sheet" : "All projects"} onClick={leave} />
        <div className="topbar-title" data-tauri-drag-region>
          {project ? "Edit site" : "New site"}
        </div>
        <div className="topbar-spacer" data-tauri-drag-region />
        <span className="topbar-subtle">Netherlands only for now</span>
        <Button icon="undo" tip="Undo" shortcut="mod+z" disabled={!history.canUndo} onClick={history.undo} />
        <Button icon="redo" tip="Redo" shortcut="mod+shift+z" disabled={!history.canRedo} onClick={history.redo} />
      </header>

      <div className="site-body">
        <div className="map-wrap">
          <div ref={container} className="map" />
          {statusText && <div className="draw-status">{statusText}</div>}
          <ZoomControl
            fitTip={area ? "Fit the site" : "Show the Netherlands"}
            onZoomIn={() => map?.zoomIn()}
            onZoomOut={() => map?.zoomOut()}
            onFit={resetView}
          />
          {drawing.live && (
            <div className="live-label tabular" style={{ left: drawing.live.x + 14, top: drawing.live.y + 14 }}>
              {drawing.live.text}
            </div>
          )}
        </div>

        <SitePanel
          onPick={pickPlace}
          tool={tool}
          onTool={(next) => setTool(tool === next ? "pan" : next)}
          name={name}
          onName={rename}
          area={area}
          size={size}
          onRadius={setRadius}
          onRadiusSeal={history.seal}
          onShape={changeShape}
          onDelete={deleteArea}
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

// The main workspace: the diagrams on the left, the sheet in the middle and
// the inspector on the right.

import { useCallback, useEffect, useRef, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import type { Navigate } from "../App";
import { useCommandHandler } from "../lib/commands";
import { closeProject, errorText, onLibraryChanged, reloadProject, renameProject } from "../lib/ipc";
import { usePlainKeys } from "../lib/keys";
import { shortcutLabel } from "../lib/platform";
import { readPreference, writePreference } from "../lib/storage";
import { chosenVariant, drawnName } from "../lib/diagrams";
import type { Catalog, Palette, ProjectView } from "../lib/types";
import { useLookHistory } from "../lib/history";
import { Button } from "../ui/Button";
import { EditableText } from "../ui/EditableText";
import { Problem } from "../ui/Problem";
import { useToast } from "../ui/Toasts";
import { Canvas, type CanvasControls } from "./Canvas";
import { ExportDialog } from "./ExportDialog";
import { DiagramColumn } from "./DiagramColumn";
import { Inspector } from "./inspector/Inspector";
import { useAutosave } from "./useAutosave";
import { usePreview } from "./usePreview";
import { useThumbnails } from "./useThumbnails";
import { TextOverlay } from "./text/TextOverlay";
import { useTextTool } from "./text/useTextTool";
import { PlateDropLine } from "./plates/PlateDropLine";
import { usePlateDrag } from "./plates/usePlateDrag";
import { ColumnResizer, DIAGRAM_COLUMN, INSPECTOR_COLUMN } from "./columns/ColumnResizer";
import { PaletteSwatches, type Swatch } from "../ui/ColorPicker";
import { ReportsWorkspace } from "../report/ReportsWorkspace";

// File events come in bursts; read the project once they settle.
const RELOAD_DELAY_MS = 400;

type Props = {
  project: ProjectView;
  catalog: Catalog | null;
  catalogError: string | null;
  onRetryCatalog: () => void;
  navigate: Navigate;
};

export function SheetStep({ project: initial, catalog, catalogError, onRetryCatalog, navigate }: Props) {
  const [project, setProject] = useState(initial);
  const history = useLookHistory(initial.look);
  const look = history.look;
  const [revision, setRevision] = useState(0);
  const [isInspectorOpen, setInspectorOpen] = useState(() => readPreference("inspectorOpen", true));
  const [isDiagramsOpen, setDiagramsOpen] = useState(() => readPreference("diagramsOpen", true));
  const [isExportOpen, setExportOpen] = useState(false);
  // The report editor opens over the sheet until reports become a sheet tab.
  const [isReportsOpen, setReportsOpen] = useState(false);
  const [zoom, setZoom] = useState(1);
  const canvas = useRef<CanvasControls>(null);
  const toast = useToast();

  const isReady = catalog !== null && project.has_scene;
  const preview = usePreview(look, revision, isReady);
  const thumbnails = useThumbnails(look, catalog?.diagrams ?? [], revision, isReady);
  const reportSaveError = useCallback((message: string) => toast(`Couldn't save. ${message}`, { isError: true }), [toast]);
  const autosave = useAutosave(look, reportSaveError);

  const diagrams = catalog?.diagrams ?? [];
  const themes = catalog?.themes ?? [];
  const diagram = diagrams.find((entry) => entry.id === look.diagram_id);
  const variant = diagram ? chosenVariant(diagram, look.variant) : null;
  const diagramName = diagram ? drawnName(diagram, look.variant) : "";
  const sheetCount = diagrams.reduce((count, entry) => count + Math.max(1, entry.variants.length), 0);
  const theme = themes.find((entry) => entry.id === look.theme_id);
  const swatches = theme ? paletteSwatches(theme.palette) : [];
  const textTool = useTextTool(look, history, canvas, preview.svg, !isExportOpen && isReady);
  const plateDrag = usePlateDrag(look, history, preview.svg);

  // A new diagram starts on its first variant.
  const selectDiagram = (diagram_id: string) => {
    if (diagram_id !== look.diagram_id) history.change({ ...look, diagram_id, variant: null });
  };
  const selectVariant = (variantId: string) => history.change({ ...look, variant: variantId });

  const stepDiagram = (direction: 1 | -1) => {
    const index = diagrams.findIndex((entry) => entry.id === look.diagram_id);
    const next = diagrams[index + direction];
    if (next) selectDiagram(next.id);
  };

  const stepVariant = (direction: 1 | -1) => {
    if (!diagram || !variant) return;
    const index = diagram.variants.indexOf(variant);
    const next = diagram.variants[index + direction];
    if (next) selectVariant(next.id);
  };

  const stepTheme = (direction: 1 | -1) => {
    if (themes.length === 0) return;
    const index = themes.findIndex((theme) => theme.id === look.theme_id);
    const next = themes[(index + direction + themes.length) % themes.length];
    history.change({ ...look, theme_id: next.id });
  };

  const toggleInspector = () => {
    setInspectorOpen(!isInspectorOpen);
    writePreference("inspectorOpen", !isInspectorOpen);
  };

  const toggleDiagrams = () => {
    setDiagramsOpen(!isDiagramsOpen);
    writePreference("diagramsOpen", !isDiagramsOpen);
  };

  const leave = async () => {
    await autosave.flush();
    await closeProject().catch(() => undefined);
    navigate.home();
  };

  const rename = async (name: string) => {
    setProject({ ...project, name });
    try {
      await renameProject(project.id, name);
    } catch (error) {
      toast(`Couldn't rename the project. ${errorText(error)}`, { isError: true });
    }
  };

  // An assistant edited this project through the MCP server: show its version,
  // unless the user is mid-edit, in which case their next save wins and the
  // assistant's version stays in the backups.
  const latest = useRef({ autosave, history, project });
  latest.current = { autosave, history, project };
  useEffect(() => {
    let timer = 0;
    const reload = async () => {
      const { autosave: saving, history: undo } = latest.current;
      if (saving.hasUnsaved) {
        toast("An assistant changed this project while you were editing. Your version was kept.");
        return;
      }
      try {
        const fresh = await reloadProject();
        const before = latest.current.history.look;
        saving.markSaved(fresh.look);
        undo.replace(fresh.look);
        setProject(fresh);
        setRevision((value) => value + 1);
        toast("Updated by assistant", {
          action: { label: "Undo", run: () => latest.current.history.replace(before) },
        });
      } catch (error) {
        toast(`Couldn't load the assistant's changes. ${errorText(error)}`, { isError: true });
      }
    };
    const unlisten = onLibraryChanged((id) => {
      if (id !== latest.current.project.id) return;
      window.clearTimeout(timer);
      timer = window.setTimeout(reload, RELOAD_DELAY_MS);
    });
    return () => {
      window.clearTimeout(timer);
      unlisten.then((stop) => stop());
    };
  }, [toast]);

  // Write the last change before the window closes.
  useEffect(() => {
    const unlisten = getCurrentWindow().onCloseRequested(async () => {
      await latest.current.autosave.flush();
    });
    return () => {
      unlisten.then((stop) => stop());
    };
  }, []);

  // Toasts sit centred on the sheet, between the two columns. A narrow window
  // lays the diagram column over the sheet, so the toast does not move for it there.
  useEffect(() => {
    const root = document.documentElement.style;
    root.setProperty("--toast-right", isInspectorOpen ? "var(--inspector-width)" : "0px");
    root.setProperty("--toast-left", isDiagramsOpen ? "var(--diagram-column-pushes)" : "0px");
    return () => {
      root.removeProperty("--toast-right");
      root.removeProperty("--toast-left");
    };
  }, [isInspectorOpen, isDiagramsOpen]);

  useCommandHandler((id) => {
    if (id === "undo") {
      history.undo();
      return true;
    }
    if (id === "redo") {
      history.redo();
      return true;
    }
    if (id === "export") {
      setExportOpen(true);
      return true;
    }
    if (id === "zoom-in") canvas.current?.zoomIn();
    if (id === "zoom-out") canvas.current?.zoomOut();
    if (id === "zoom-fit") canvas.current?.fit();
    if (id === "inspector") toggleInspector();
    if (id === "diagrams") toggleDiagrams();
    if (id === "add-text" && isReady) textTool.addText();
    if (id === "home") leave();
    return ["zoom-in", "zoom-out", "zoom-fit", "inspector", "diagrams", "home", "add-text"].includes(id);
  });

  usePlainKeys((key, event) => {
    if (/^[1-9]$/.test(key)) {
      const target = diagrams[Number(key) - 1];
      if (target) selectDiagram(target.id);
    }
    if (key === "[") stepTheme(-1);
    if (key === "]") stepTheme(1);
    if (key.toLowerCase() === "i" && !event.repeat) toggleInspector();
    if (key.toLowerCase() === "d" && !event.repeat) toggleDiagrams();
    // With a text box selected the arrows nudge it instead; see useTextTool.
    const steps: Record<string, () => void> = {
      ArrowUp: () => stepDiagram(-1),
      ArrowDown: () => stepDiagram(1),
      ArrowLeft: () => stepVariant(-1),
      ArrowRight: () => stepVariant(1),
    };
    if (steps[key] && textTool.selectedId === null) {
      event.preventDefault();
      steps[key]();
    }
  }, !isExportOpen && !isReportsOpen);

  const saveText = { saved: "Saved", pending: "Edited", saving: "Saving…", failed: "Not saved" }[autosave.status];

  return (
    <div className="sheet-step">
      <header className="topbar" data-tauri-drag-region>
        <Button icon="back" tip="All projects" shortcut="mod+shift+h" onClick={leave} />
        <Button icon="sidebarLeft" tip="Diagrams" shortcut="d" isActive={isDiagramsOpen} onClick={toggleDiagrams} />
        <div className="topbar-title" data-tauri-drag-region>
          <EditableText className="project-name" value={project.name} onCommit={rename} />
          {diagram && <span className="topbar-crumb">{diagramName}</span>}
        </div>
        <div className="topbar-spacer" data-tauri-drag-region />
        <span className={autosave.status === "failed" ? "save-status is-failed" : "save-status"}>{saveText}</span>
        <Button icon="undo" tip="Undo" shortcut="mod+z" disabled={!history.canUndo} onClick={history.undo} />
        <Button icon="redo" tip="Redo" shortcut="mod+shift+z" disabled={!history.canRedo} onClick={history.redo} />
        <span className="topbar-divider" />
        <Button icon="text" tip="Add text" shortcut="t" disabled={!isReady} onClick={textTool.addText} />
        <Button icon="text" tip="Lay out a report of this site" disabled={!catalog} onClick={() => setReportsOpen(true)}>
          Reports
        </Button>
        <Button icon="map" tip="Draw the site again" onClick={() => navigate.editSite(project)}>
          Edit site
        </Button>
        <Button icon="export" variant="secondary" tip="Export" shortcut="mod+e" disabled={!isReady} onClick={() => setExportOpen(true)}>
          Export
        </Button>
        <Button icon="sidebar" tip="Inspector" shortcut="i" isActive={isInspectorOpen} onClick={toggleInspector} />
      </header>

      <PaletteSwatches.Provider value={swatches}>
      <div className="sheet-body">
        <div className={isDiagramsOpen ? "diagram-slot is-open" : "diagram-slot"}>
          <DiagramColumn
            diagrams={diagrams}
            selectedId={look.diagram_id}
            variantId={look.variant}
            thumbnails={thumbnails}
            onSelect={selectDiagram}
            onVariant={selectVariant}
          />
          <ColumnResizer column={DIAGRAM_COLUMN} />
        </div>
        <div className="sheet-main">
          <div className="canvas-wrap">
            <Canvas
              svg={preview.svg}
              controls={canvas}
              onZoom={setZoom}
              onPaperPointerDown={(event, view) => textTool.onPaperPointerDown(event, view) || plateDrag.onPaperPointerDown(event, view)}
              onPaperDoubleClick={textTool.onPaperDoubleClick}
              overlay={(view) =>
                preview.svg && (
                  <>
                    <PlateDropLine view={view} drag={plateDrag.drag} />
                    <TextOverlay
                      view={view}
                      svg={preview.svg}
                      look={look}
                      selectedId={textTool.selectedId}
                      editingId={textTool.editingId}
                      drag={textTool.drag}
                      diagramName={diagramName}
                      onLook={history.change}
                      onSeal={history.seal}
                      onEditEnd={() => textTool.setEditingId(null)}
                      onDeleted={() => textTool.setSelectedId(null)}
                    />
                  </>
                )
              }
            />
            {!preview.svg && <CanvasMessage catalogError={catalogError} previewError={preview.error} onRetry={onRetryCatalog} hasScene={project.has_scene} />}
            {preview.svg && preview.error && (
              <div className="canvas-error" role="alert">
                {preview.error}
              </div>
            )}
            {preview.isSlow && <div className="render-hint" aria-label="Rendering" />}
            <div className="zoom-control">
              <Button icon="zoomOut" tip="Zoom out" shortcut="mod+-" tipSide="above" onClick={() => canvas.current?.zoomOut()} />
              <button type="button" className="zoom-value tabular" data-tip="Fit sheet" data-tip-key={shortcutLabel("mod+0")} data-tip-side="above" onClick={() => canvas.current?.fit()}>
                {Math.round(zoom * 100)}%
              </button>
              <Button icon="zoomIn" tip="Zoom in" shortcut="mod+=" tipSide="above" onClick={() => canvas.current?.zoomIn()} />
            </div>
          </div>
        </div>
        <div className={isInspectorOpen ? "inspector-slot is-open" : "inspector-slot"}>
          <ColumnResizer column={INSPECTOR_COLUMN} />
          {catalog && <Inspector catalog={catalog} project={project} look={look} onLook={history.change} onSeal={history.seal} />}
        </div>
      </div>
      </PaletteSwatches.Provider>

      {isReportsOpen && catalog && (
        <ReportsWorkspace
          project={{ ...project, look }}
          catalog={catalog}
          onClose={() => setReportsOpen(false)}
          onSaved={(reports) => setProject((current) => ({ ...current, reports }))}
        />
      )}

      {isExportOpen && diagram && (
        <ExportDialog
          look={look}
          diagramName={diagramName}
          variantId={variant?.id ?? null}
          sheetCount={sheetCount}
          onClose={() => setExportOpen(false)}
        />
      )}
    </div>
  );
}

function CanvasMessage({
  catalogError,
  previewError,
  onRetry,
  hasScene,
}: {
  catalogError: string | null;
  previewError: string | null;
  onRetry: () => void;
  hasScene: boolean;
}) {
  if (!hasScene) {
    return (
      <div className="canvas-message">
        <Problem title="This site hasn't been fetched yet." detail="Use Edit site to draw it and fetch the data." />
      </div>
    );
  }
  if (catalogError) {
    return (
      <div className="canvas-message">
        <Problem title="Diagrams and palettes aren't available." detail={catalogError}>
          <Button variant="secondary" onClick={onRetry}>
            Try again
          </Button>
        </Problem>
      </div>
    );
  }
  if (previewError) {
    return (
      <div className="canvas-message">
        <Problem title="The sheet couldn't be drawn." detail={previewError} />
      </div>
    );
  }
  return <div className="canvas-message"><div className="canvas-loading" aria-label="Rendering" /></div>;
}

// The palette roles a colour picker offers on this sheet, in one place.
const SWATCH_ROLES: (keyof Palette)[] = ["ink", "paper", "figure", "accent", "water", "green", "muted", "ground"];

/** The theme's palette as picker swatches. Picking one for text keeps the role, so it follows theme changes. */
function paletteSwatches(palette: Palette): Swatch[] {
  return SWATCH_ROLES.map((role) => ({ value: `@${role}`, colour: palette[role], label: role[0].toUpperCase() + role.slice(1) }));
}

// The main workspace: the sheet in the middle, the inspector on the right,
// and the series of diagrams along the bottom.

import { useCallback, useEffect, useRef, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import type { Navigate } from "../App";
import { useCommandHandler } from "../lib/commands";
import { closeProject, errorText, onLibraryChanged, reloadProject, renameProject } from "../lib/ipc";
import { usePlainKeys } from "../lib/keys";
import { shortcutLabel } from "../lib/platform";
import { readPreference, writePreference } from "../lib/storage";
import type { Catalog, ProjectView } from "../lib/types";
import { useLookHistory } from "../lib/history";
import { Button } from "../ui/Button";
import { EditableText } from "../ui/EditableText";
import { Problem } from "../ui/Problem";
import { useToast } from "../ui/Toasts";
import { Canvas, type CanvasControls } from "./Canvas";
import { ExportDialog } from "./ExportDialog";
import { Filmstrip } from "./Filmstrip";
import { Inspector } from "./inspector/Inspector";
import { useAutosave } from "./useAutosave";
import { usePreview } from "./usePreview";
import { useThumbnails } from "./useThumbnails";

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
  const [isExportOpen, setExportOpen] = useState(false);
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

  const selectDiagram = (diagram_id: string) => history.change({ ...look, diagram_id });

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

  // Toasts sit centred on the workspace, not the whole window.
  useEffect(() => {
    const root = document.documentElement.style;
    root.setProperty("--toast-right", isInspectorOpen ? "var(--inspector-width)" : "0px");
    root.setProperty("--toast-bottom", "calc(var(--filmstrip-height) + 20px)");
    return () => {
      root.removeProperty("--toast-right");
      root.removeProperty("--toast-bottom");
    };
  }, [isInspectorOpen]);

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
    if (id === "home") leave();
    return ["zoom-in", "zoom-out", "zoom-fit", "inspector", "home"].includes(id);
  });

  usePlainKeys((key, event) => {
    if (/^[1-9]$/.test(key)) {
      const target = diagrams[Number(key) - 1];
      if (target) selectDiagram(target.id);
    }
    if (key === "[") stepTheme(-1);
    if (key === "]") stepTheme(1);
    if (key.toLowerCase() === "i" && !event.repeat) toggleInspector();
  }, !isExportOpen);

  const saveText = { saved: "Saved", pending: "Edited", saving: "Saving…", failed: "Not saved" }[autosave.status];

  return (
    <div className="sheet-step">
      <header className="topbar" data-tauri-drag-region>
        <Button icon="back" tip="All projects" shortcut="mod+shift+h" onClick={leave} />
        <div className="topbar-title" data-tauri-drag-region>
          <EditableText className="project-name" value={project.name} onCommit={rename} />
          {diagram && <span className="topbar-crumb">{diagram.name}</span>}
        </div>
        <div className="topbar-spacer" data-tauri-drag-region />
        <span className={autosave.status === "failed" ? "save-status is-failed" : "save-status"}>{saveText}</span>
        <Button icon="undo" tip="Undo" shortcut="mod+z" disabled={!history.canUndo} onClick={history.undo} />
        <Button icon="redo" tip="Redo" shortcut="mod+shift+z" disabled={!history.canRedo} onClick={history.redo} />
        <span className="topbar-divider" />
        <Button icon="map" tip="Draw the site again" onClick={() => navigate.editSite(project)}>
          Edit site
        </Button>
        <Button icon="export" variant="secondary" tip="Export" shortcut="mod+e" disabled={!isReady} onClick={() => setExportOpen(true)}>
          Export
        </Button>
        <Button icon="sidebar" tip="Inspector" shortcut="i" isActive={isInspectorOpen} onClick={toggleInspector} />
      </header>

      <div className="sheet-body">
        <div className="sheet-main">
          <div className="canvas-wrap">
            <Canvas svg={preview.svg} controls={canvas} onZoom={setZoom} />
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
          <Filmstrip diagrams={diagrams} selectedId={look.diagram_id} thumbnails={thumbnails} onSelect={selectDiagram} />
        </div>
        <div className={isInspectorOpen ? "inspector-slot is-open" : "inspector-slot"}>
          {catalog && <Inspector catalog={catalog} project={project} look={look} thumbnails={thumbnails} onLook={history.change} onSeal={history.seal} />}
        </div>
      </div>

      {isExportOpen && diagram && (
        <ExportDialog look={look} diagramName={diagram.name} diagramCount={diagrams.length} onClose={() => setExportOpen(false)} />
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
        <Problem title="Diagrams and themes aren't available." detail={catalogError}>
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

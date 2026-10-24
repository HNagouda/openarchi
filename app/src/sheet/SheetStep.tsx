// The main workspace: the diagrams on the left, the sheet in the middle with
// its tabs underneath, and the inspector on the right.

import { useCallback, useEffect, useRef, useState } from "react";
import type { Navigate } from "../App";
import { useCommandHandler } from "../lib/commands";
import { closeProject, errorText, renameProject } from "../lib/ipc";
import { usePlainKeys } from "../lib/keys";
import { shortcutLabel } from "../lib/platform";
import { readPreference, writePreference } from "../lib/storage";
import { chosenVariant, drawnName } from "../lib/diagrams";
import type { Catalog, ProjectView } from "../lib/types";
import { Button } from "../ui/Button";
import { EditableText } from "../ui/EditableText";
import { CanvasMessage } from "./CanvasMessage";
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
import { ColumnResizer, DIAGRAM_COLUMN, INSPECTOR_COLUMN } from "./columns/ColumnResizer";
import { PaletteSwatches } from "../ui/ColorPicker";
import { paletteSwatches } from "./paletteSwatches";
import { PinOverlay } from "./pins/PinOverlay";
import { usePinTool } from "./pins/usePinTool";
import { SelectionDialog } from "./selection/SelectionDialog";
import { SheetTabs } from "./tabs/SheetTabs";
import { useTabKeys } from "./tabs/useTabKeys";
import { saveWorkbook, useWorkbook } from "./tabs/useWorkbook";
import { useProjectSync } from "./useProjectSync";
import { ReportsWorkspace } from "../report/ReportsWorkspace";
import { useLibraryPreviews } from "../report/previews/useLibraryPreviews";

type Props = {
  project: ProjectView;
  catalog: Catalog | null;
  catalogError: string | null;
  /** Reads the catalog again; also after a palette was saved or deleted. */
  onRetryCatalog: () => Promise<unknown> | void;
  navigate: Navigate;
};

export function SheetStep({ project: initial, catalog, catalogError, onRetryCatalog, navigate }: Props) {
  const [project, setProject] = useState(initial);
  const history = useWorkbook(initial);
  const { book, look } = history;
  const [isAreaOpen, setAreaOpen] = useState(false);
  const [revision, setRevision] = useState(0);
  const [isInspectorOpen, setInspectorOpen] = useState(() => readPreference("inspectorOpen", true));
  const [isDiagramsOpen, setDiagramsOpen] = useState(() => readPreference("diagramsOpen", true));
  const [isExportOpen, setExportOpen] = useState(false);
  // The report editor opens over the sheet; the tabs below hold sheets only.
  const [isReportsOpen, setReportsOpen] = useState(false);
  const [zoom, setZoom] = useState(1);
  const canvas = useRef<CanvasControls>(null);
  const toast = useToast();

  const isReady = catalog !== null && project.has_scene;
  const preview = usePreview(look, book.pins, revision, isReady);
  const thumbnails = useThumbnails(look, catalog?.diagrams ?? [], revision, isReady);
  // The report library, gathered in the background so it is ready when Reports opens.
  const libraryPreviews = useLibraryPreviews(project, book.sheets[0].theme_id, isReady);
  const reportSaveError = useCallback((message: string) => toast(`Couldn't save. ${message}`, { isError: true }), [toast]);
  const autosave = useAutosave(book, saveWorkbook, reportSaveError);

  const diagrams = catalog?.diagrams ?? [];
  const themes = catalog?.themes ?? [];
  const diagram = diagrams.find((entry) => entry.id === look.diagram_id);
  const variant = diagram ? chosenVariant(diagram, look.variant) : null;
  const diagramName = diagram ? drawnName(diagram, look.variant) : "";
  const sheetCount = diagrams.reduce((count, entry) => count + Math.max(1, entry.variants.length), 0);
  const theme = themes.find((entry) => entry.id === look.theme_id);
  const swatches = theme ? paletteSwatches(theme.palette) : [];
  const isEditable = !isExportOpen && !isAreaOpen && !isReportsOpen && isReady;
  const textTool = useTextTool(look, { change: history.changeLook, seal: history.seal }, canvas, preview.svg, isEditable);
  const pinTool = usePinTool({
    pins: book.pins,
    onPins: history.changePins,
    canvas,
    svg: preview.svg,
    isEnabled: isEditable,
    onRefused: (message) => toast(message),
  });
  useTabKeys(book.sheets.length, history.index, history.setActive, !isExportOpen && !isAreaOpen && !isReportsOpen);

  // A new diagram starts on its first variant.
  const selectDiagram = (diagram_id: string) => {
    if (diagram_id !== look.diagram_id) history.changeLook({ ...look, diagram_id, variant: null });
  };
  const selectVariant = (variantId: string) => history.changeLook({ ...look, variant: variantId });

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
    history.changeLook({ ...look, theme_id: next.id });
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

  useProjectSync({
    projectId: project.id,
    autosave,
    history,
    onReloaded: (fresh) => {
      setProject(fresh);
      setRevision((value) => value + 1);
    },
  });

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
    // The area dialog has no undo of its own; the sheet's must not change under it.
    if ((id === "undo" || id === "redo") && isAreaOpen) return true;
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
    if (steps[key] && textTool.selectedId === null && pinTool.selectedId === null) {
      event.preventDefault();
      steps[key]();
    }
  }, !isExportOpen && !isAreaOpen && !isReportsOpen);

  const saveText = { saved: "Saved", pending: "Edited", saving: "Saving…", failed: "Not saved" }[autosave.status];

  return (
    <div className="sheet-step">
      {/* Under the report editor the sheet is out of reach of Tab and the screen reader. */}
      <header className="topbar" data-tauri-drag-region inert={isReportsOpen}>
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
        <Button icon="report" tip="Lay out a report of this site" disabled={!catalog} onClick={() => setReportsOpen(true)}>
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
      <div className="sheet-body" inert={isReportsOpen}>
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
              onPaperPointerDown={(event, view) => {
                // Pins sit on top of the drawing, so they take the press first.
                if (pinTool.onPaperPointerDown(event, view)) {
                  textTool.setSelectedId(null);
                  return true;
                }
                return textTool.onPaperPointerDown(event, view);
              }}
              onPaperDoubleClick={(event, view) => pinTool.onPaperDoubleClick(event, view) || textTool.onPaperDoubleClick(event, view)}
              overlay={(view) =>
                preview.svg && (
                  <>
                    <PinOverlay
                      view={view}
                      svg={preview.svg}
                      pins={book.pins}
                      selectedId={pinTool.selectedId}
                      editingId={pinTool.editingId}
                      drag={pinTool.drag}
                      onPins={history.changePins}
                      onSeal={history.seal}
                      onEdit={() => pinTool.setEditingId(pinTool.selectedId)}
                      onEditEnd={() => pinTool.setEditingId(null)}
                      onDeleted={() => pinTool.setSelectedId(null)}
                    />
                    <TextOverlay
                      view={view}
                      svg={preview.svg}
                      look={look}
                      selectedId={textTool.selectedId}
                      editingId={textTool.editingId}
                      drag={textTool.drag}
                      diagramName={diagramName}
                      onLook={history.changeLook}
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
          <SheetTabs
            book={book}
            index={history.index}
            look={look}
            isReady={isReady}
            onSelect={history.setActive}
            onAdd={history.addSheet}
            onRename={history.renameSheet}
            onDuplicate={history.duplicateSheet}
            onDelete={history.deleteSheet}
            onMove={history.moveSheet}
            onLook={history.changeLook}
            onDropPin={pinTool.dropAt}
            onPlacePin={pinTool.placeInView}
            onEditArea={() => setAreaOpen(true)}
          />
        </div>
        <div className={isInspectorOpen ? "inspector-slot is-open" : "inspector-slot"}>
          <ColumnResizer column={INSPECTOR_COLUMN} />
          {catalog && (
            <Inspector catalog={catalog} project={project} look={look} onLook={history.changeLook} onSeal={history.seal} onCatalogChanged={onRetryCatalog} />
          )}
        </div>
      </div>
      </PaletteSwatches.Provider>

      {isReportsOpen && catalog && (
        <ReportsWorkspace
          // Sheet blocks follow the first sheet, as the core draws them.
          project={{ ...project, look: book.sheets[0] }}
          catalog={catalog}
          previews={libraryPreviews}
          sheetThumbnails={look === book.sheets[0] ? thumbnails : undefined}
          onClose={() => setReportsOpen(false)}
          onSaved={(reports) => setProject((current) => ({ ...current, reports }))}
        />
      )}

      {isExportOpen && diagram && (
        <ExportDialog
          look={look}
          pins={book.pins}
          diagramName={diagramName}
          variantId={variant?.id ?? null}
          sheetCount={sheetCount}
          onClose={() => setExportOpen(false)}
        />
      )}
      {isAreaOpen && (
        <SelectionDialog
          site={project.area}
          selection={look.selection ?? null}
          onApply={(selection) => {
            setAreaOpen(false);
            history.applySelection(selection);
          }}
          onClose={() => setAreaOpen(false)}
        />
      )}
    </div>
  );
}

// The main workspace: the diagrams on the left, the sheet in the middle with
// its tabs underneath, and the inspector on the right.

import { useCallback, useRef, useState } from "react";
import type { Navigate } from "../App";
import { closeProject, errorText, renameProject } from "../lib/ipc";
import { chosenVariant, drawnName } from "../lib/diagrams";
import type { Catalog, Diagram, ProjectView } from "../lib/types";
import { useToast } from "../ui/Toasts";
import type { CanvasControls } from "./Canvas";
import { ExportDialog } from "./ExportDialog";
import { DiagramColumn } from "./DiagramColumn";
import { Inspector } from "./inspector/Inspector";
import { useAutosave } from "./useAutosave";
import { usePreview } from "./usePreview";
import { useThumbnails } from "./useThumbnails";
import { useDiagramStepping } from "./useDiagramStepping";
import { useLiveValue } from "./liveValue";
import { SheetCanvas } from "./SheetCanvas";
import { SheetTopbar } from "./SheetTopbar";
import { useSheetColumns } from "./useSheetColumns";
import { useSheetKeys } from "./useSheetKeys";
import { usePaperTools } from "./usePaperTools";
import { ColumnResizer, DIAGRAM_COLUMN, INSPECTOR_COLUMN } from "./columns/ColumnResizer";
import { PaletteSwatches } from "../ui/ColorPicker";
import { paletteSwatches } from "./paletteSwatches";
import { SheetTabs } from "./tabs/SheetTabs";
import { useTabKeys } from "./tabs/useTabKeys";
import { saveWorkbook, useWorkbook } from "./tabs/useWorkbook";
import { useProjectSync } from "./useProjectSync";
import { ReportsWorkspace } from "../report/ReportsWorkspace";
import { useLibraryPreviews } from "../report/previews/useLibraryPreviews";

// One empty list while the catalog loads, so the memoized diagram column and the thumbnails do not see a new one every render.
const NO_DIAGRAMS: Diagram[] = [];

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
  const [revision, setRevision] = useState(0);
  const columns = useSheetColumns();
  const [isExportOpen, setExportOpen] = useState(false);
  // The report editor opens over the sheet; the tabs below hold sheets only.
  const [isReportsOpen, setReportsOpen] = useState(false);
  const zoom = useLiveValue(1);
  const canvas = useRef<CanvasControls>(null);
  const toast = useToast();

  const isReady = catalog !== null && project.has_scene;
  const preview = usePreview(look, book.pins, book.highlights, revision, isReady);
  const thumbnails = useThumbnails(look, catalog?.diagrams ?? NO_DIAGRAMS, revision, isReady);
  // The report library, gathered in the background so it is ready when Reports opens.
  const libraryPreviews = useLibraryPreviews(project, book.sheets[0].theme_id, isReady);
  const reportSaveError = useCallback((message: string) => toast(`Couldn't save. ${message}`, { isError: true }), [toast]);
  const autosave = useAutosave(book, saveWorkbook, reportSaveError);

  const diagrams = catalog?.diagrams ?? NO_DIAGRAMS;
  const themes = catalog?.themes ?? [];
  const diagram = diagrams.find((entry) => entry.id === look.diagram_id);
  const variant = diagram ? chosenVariant(diagram, look.variant) : null;
  const diagramName = diagram ? drawnName(diagram, look.variant) : "";
  const sheetCount = diagrams.reduce((count, entry) => count + Math.max(1, entry.variants.length), 0);
  const theme = themes.find((entry) => entry.id === look.theme_id);
  const swatches = theme ? paletteSwatches(theme.palette) : [];
  const isEditable = !isExportOpen && !isReportsOpen && isReady;
  const { tools, highlights } = usePaperTools({ history, diagrams, canvas, svg: preview.svg, isEditable, isReady, toast });
  useTabKeys(book.sheets.length, history.index, history.setActive, !isExportOpen && !isReportsOpen);
  const stepping = useDiagramStepping(look, diagrams, themes, history.changeLook);

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

  useSheetKeys({
    canvas,
    columns,
    stepping,
    isReady,
    isEnabled: !isExportOpen && !isReportsOpen,
    isNothingSelected: tools.text.selectedId === null && tools.pin.selectedId === null && tools.highlight.selectedId === null,
    onUndo: history.undo,
    onRedo: history.redo,
    onExport: () => setExportOpen(true),
    onAddText: tools.text.addText,
    onLeave: leave,
  });

  return (
    <div className="sheet-step">
      <SheetTopbar
        projectName={project.name}
        diagramName={diagram ? diagramName : null}
        saveStatus={autosave.status}
        canUndo={history.canUndo}
        canRedo={history.canRedo}
        isReady={isReady}
        hasCatalog={catalog !== null}
        isDiagramsOpen={columns.isDiagramsOpen}
        isInspectorOpen={columns.isInspectorOpen}
        isInert={isReportsOpen}
        onRename={rename}
        onLeave={leave}
        onUndo={history.undo}
        onRedo={history.redo}
        onAddText={tools.text.addText}
        onReports={() => setReportsOpen(true)}
        onEditSite={() => navigate.editSite(project)}
        onExport={() => setExportOpen(true)}
        onToggleDiagrams={columns.toggleDiagrams}
        onToggleInspector={columns.toggleInspector}
      />

      <PaletteSwatches.Provider value={swatches}>
      <div className="sheet-body" inert={isReportsOpen}>
        <div className={columns.isDiagramsOpen ? "diagram-slot is-open" : "diagram-slot"}>
          <DiagramColumn
            diagrams={diagrams}
            selectedId={look.diagram_id}
            variantId={look.variant}
            thumbnails={thumbnails}
            onSelect={stepping.selectDiagram}
            onVariant={stepping.selectVariant}
          />
          <ColumnResizer column={DIAGRAM_COLUMN} />
        </div>
        <div className="sheet-main">
          <SheetCanvas
            preview={preview}
            canvas={canvas}
            zoom={zoom}
            tools={tools}
            highlightOverlay={highlights.overlay}
            look={look}
            pins={book.pins}
            diagramName={diagramName}
            onLook={history.changeLook}
            onPins={history.changePins}
            onSeal={history.seal}
            hasScene={project.has_scene}
            catalogError={catalogError}
            onRetryCatalog={onRetryCatalog}
            onFetchAgain={() => navigate.editSite(project)}
          />
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
            onDropPin={tools.pin.dropAt}
            onPlacePin={tools.pin.placeInView}
            isDrawingSection={tools.section.isDrawing}
            onSectionTool={tools.section.toggle}
            highlightTools={highlights.tabTools}
          />
        </div>
        <div className={columns.isInspectorOpen ? "inspector-slot is-open" : "inspector-slot"}>
          <ColumnResizer column={INSPECTOR_COLUMN} />
          {catalog && (
            <Inspector
              catalog={catalog}
              project={project}
              look={look}
              onLook={history.changeLook}
              onSeal={history.seal}
              onCatalogChanged={onRetryCatalog}
              viewPoint={{ pins: book.pins, isPlacing: tools.viewPoint.isPlacing, onPlace: tools.viewPoint.toggle }}
            />
          )}
        </div>
      </div>
      </PaletteSwatches.Provider>

      {isReportsOpen && catalog && (
        <ReportsWorkspace
          project={project}
          // Sheet blocks follow the first sheet, as the core draws them.
          sheetLook={book.sheets[0]}
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
          highlights={book.highlights}
          diagramName={diagramName}
          variantId={variant?.id ?? null}
          sheetCount={sheetCount}
          onClose={() => setExportOpen(false)}
        />
      )}
    </div>
  );
}

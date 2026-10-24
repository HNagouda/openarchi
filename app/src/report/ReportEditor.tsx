// The report editor: the block library on the left, the pages under their
// rulers in the middle and the inspector on the right. It edits one report
// and hands every change to its owner through `onChange`, which keeps the undo
// history, so it can sit in any container: a screen of its own now, a sheet tab later.

import { useCallback, useEffect, useImperativeHandle, useMemo, useRef, useState, type Ref } from "react";
import { shortcutLabel } from "../lib/platform";
import { hasModifier, isTyping, useKeyDown } from "../lib/keys";
import type { Catalog, ProjectView, Report, ReportBlock, TextStyle } from "../lib/types";
import { ColumnResizer, REPORT_INSPECTOR_COLUMN, REPORT_LIBRARY_COLUMN } from "../sheet/columns/ColumnResizer";
import { useThumbnails } from "../sheet/useThumbnails";
import { Button } from "../ui/Button";
import { addBlock, deleteBlock, duplicateBlock, findBlock, pageSize, placedRect, restack, tidy, updateBlock } from "./edits";
import { analysisItems, ratiosItems, sheetItems, type LibraryItem } from "./library";
import { LibraryPanel, type LibraryDrag } from "./LibraryPanel";
import { PageStack } from "./PageStack";
import type { LibraryPreviews } from "./previews/libraryPreviews";
import { usePaletteLibrary } from "./previews/useLibraryPreviews";
import { ReportInspector } from "./ReportInspector";
import { Measure } from "./rulers/Measure";
import { RULER_PX, Rulers, type Extent } from "./rulers/Rulers";
import { TextBlockTools } from "./TextBlockTools";
import { textBox, textContent, type TextKind } from "./textBlocks";
import { BlockPicturesContext, type BlockPictures } from "./useBlockSvg";
import { useBlockDrag } from "./useBlockDrag";
import "./report.css";

const MM_TO_PX = 96 / 25.4;
const MIN_ZOOM = 0.25;
const MAX_ZOOM = 4;
const ZOOM_STEP = 1.25;
// Room left and right of the widest page when fitting: the ruler, the stack's
// 32 px padding on each side and a scrollbar, so fitting never scrolls sideways.
const FIT_PADDING_PX = 80 + RULER_PX;
// Room above and below a fitted page: the ruler, the stack's top padding, the
// page's header and the zoom control under it, so one whole page shows on a tall desk.
const FIT_VERTICAL_PX = 140 + RULER_PX;
// Arrow keys move a block this far, with Shift ten times as far.
const NUDGE_MM = 1;
const NUDGE_SHIFT_MM = 10;

export type ZoomControls = { zoomIn: () => void; zoomOut: () => void; fit: () => void };

type Props = {
  project: ProjectView;
  catalog: Catalog;
  report: Report;
  previews: LibraryPreviews;
  /** The sheet's own diagram thumbnails, when they show the first sheet; used while the report keeps the sheet's palette. */
  sheetThumbnails?: Record<string, string>;
  onChange: (report: Report, mergeKey?: string) => void;
  onSeal: () => void;
  /** Back to the sheet: Escape with nothing selected. */
  onLeave: () => void;
  controls?: Ref<ZoomControls>;
};

/** A block from the library on its way to a page: the page under the pointer and where it would land. */
type Incoming = { pageIndex: number; rect: { x: number; y: number; width: number; height: number } };

export function ReportEditor({ project, catalog, report, previews, sheetThumbnails, onChange, onSeal, onLeave, controls }: Props) {
  const look = project.look;
  const themeId = report.theme_id ?? look.theme_id;
  const { diagram_id: _diagram, variant: _variant, theme_id: _theme, ...sheetLook } = look;
  const lookKey = JSON.stringify(sheetLook);
  const [pageIndex, setPageIndex] = useState(0);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [incoming, setIncoming] = useState<Incoming | null>(null);
  const [zoom, setZoom] = useState(1);
  const pictures = useRef<BlockPictures>(new Map());
  const main = useRef<HTMLDivElement>(null);
  const pxPerMm = zoom * MM_TO_PX;
  const currentPage = Math.min(pageIndex, report.pages.length - 1);

  const library = usePaletteLibrary(previews, themeId);
  const ratios = library?.ratios ?? null;
  const sections = library?.sections;
  const isSheetPalette = themeId === look.theme_id && sheetThumbnails !== undefined;
  const ownThumbnails = useThumbnails({ ...look, theme_id: themeId }, catalog.diagrams, 0, project.has_scene && !isSheetPalette);
  const thumbnails = isSheetPalette ? sheetThumbnails : ownThumbnails;
  const items = useMemo(
    () => [...(ratios ? ratiosItems(ratios) : []), ...analysisItems(sections ?? []), ...sheetItems(catalog.diagrams, look.sheet)],
    [ratios, sections, catalog.diagrams, look.sheet],
  );
  const sheetPictures = useMemo(() => Object.fromEntries(Object.entries(thumbnails).map(([id, url]) => [`sheet-${id}`, url])), [thumbnails]);
  const paper = catalog.themes.find((theme) => theme.id === themeId)?.palette.paper ?? "transparent";

  const select = useCallback((id: string | null, index?: number) => {
    setSelectedId(id);
    setEditingId(null);
    if (index !== undefined) setPageIndex(index);
  }, []);
  const drag = useBlockDrag(report, pxPerMm, onChange, select);

  const fit = useCallback(() => {
    const width = main.current?.clientWidth ?? 0;
    const height = main.current?.clientHeight ?? 0;
    const widest = Math.max(...report.pages.map((page) => pageSize(page).width));
    const tallest = Math.max(...report.pages.map((page) => pageSize(page).height));
    const across = (width - FIT_PADDING_PX) / (widest * MM_TO_PX);
    const down = (height - FIT_VERTICAL_PX) / (tallest * MM_TO_PX);
    // Rounded down to a whole percent, so the fitted page never spills by a pixel.
    const fitted = Math.floor(Math.min(across, down) * 100) / 100;
    if (width > 0) setZoom(Math.min(MAX_ZOOM, Math.max(MIN_ZOOM, fitted)));
  }, [report.pages]);
  const zoomBy = (factor: number) => setZoom((current) => Math.min(MAX_ZOOM, Math.max(MIN_ZOOM, current * factor)));
  useImperativeHandle(controls, () => ({ zoomIn: () => zoomBy(ZOOM_STEP), zoomOut: () => zoomBy(1 / ZOOM_STEP), fit }), [fit]);

  // Fit the pages once, when the editor opens.
  // eslint-disable-next-line react-hooks/exhaustive-deps
  useEffect(fit, []);

  const place = (item: LibraryItem, index: number, at: { x: number; y: number }) => {
    const [next, id] = addBlock(report, index, item.content, item.size, at);
    onChange(next);
    select(id, index);
  };

  const addToCurrent = (item: LibraryItem) => {
    const size = pageSize(report.pages[currentPage]);
    place(item, currentPage, { x: size.width / 2, y: size.height / 2 });
  };

  /** The page under a point on screen and the point on it in millimetres. */
  const pageAt = (clientX: number, clientY: number) => {
    const page = document.elementFromPoint(clientX, clientY)?.closest<HTMLElement>("[data-report-page]");
    if (!page) return null;
    const box = page.getBoundingClientRect();
    return { index: Number(page.dataset.reportPage), at: { x: (clientX - box.left) / pxPerMm, y: (clientY - box.top) / pxPerMm } };
  };

  const drop = (item: LibraryItem, clientX: number, clientY: number) => {
    const target = pageAt(clientX, clientY);
    if (target) place(item, target.index, target.at);
  };

  const dragOver = (over: LibraryDrag | null) => {
    const target = over ? pageAt(over.clientX, over.clientY) : null;
    if (!over || !target) {
      setIncoming(null);
      return;
    }
    setIncoming({ pageIndex: target.index, rect: placedRect(pageSize(report.pages[target.index]), over.item.size, target.at) });
  };

  // A new text goes on the current page under what is there, selected and open for typing.
  const addText = (kind: TextKind) => {
    const page = report.pages[currentPage];
    const box = textBox(page, kind);
    const [next, id] = addBlock(report, currentPage, textContent(kind), box, { x: box.x + box.width / 2, y: box.y + box.height / 2 });
    onChange(next);
    select(id, currentPage);
    setEditingId(id);
  };

  const remove = () => {
    if (!selectedId) return;
    onChange(deleteBlock(report, selectedId));
    select(null);
  };

  const duplicate = () => {
    if (!selectedId) return;
    const [next, id] = duplicateBlock(report, selectedId);
    onChange(next);
    if (id) select(id);
  };

  const nudge = (dx: number, dy: number) => {
    const found = selectedId ? findBlock(report, selectedId) : null;
    if (!found) return;
    const size = pageSize(report.pages[found.pageIndex]);
    const x = Math.min(Math.max(found.block.x_mm + dx, 0), size.width - found.block.width_mm);
    const y = Math.min(Math.max(found.block.y_mm + dy, 0), size.height - found.block.height_mm);
    onChange(updateBlock(report, found.block.id, { x_mm: tidy(x), y_mm: tidy(y) }), `nudge:${found.block.id}`);
  };

  useKeyDown((event) => {
    if (isTyping(event.target) || document.querySelector("[data-modal-open]")) return;
    const step = event.shiftKey ? NUDGE_SHIFT_MM : NUDGE_MM;
    const arrows: Record<string, [number, number]> = { ArrowLeft: [-step, 0], ArrowRight: [step, 0], ArrowUp: [0, -step], ArrowDown: [0, step] };
    if (selectedId && arrows[event.key] && !hasModifier(event)) {
      event.preventDefault();
      nudge(...arrows[event.key]);
    }
    if (selectedId && (event.key === "Delete" || event.key === "Backspace")) remove();
    // The first Escape lets go of the block, the next one goes back to the sheet.
    if (event.key === "Escape" && selectedId) select(null);
    else if (event.key === "Escape") onLeave();
    if (event.key.toLowerCase() === "t" && !hasModifier(event) && !event.shiftKey) addText("body");
    if (selectedId && hasModifier(event) && event.key.toLowerCase() === "d") {
      event.preventDefault();
      duplicate();
    }
    if (selectedId && hasModifier(event) && (event.key === "]" || event.key === "[")) {
      event.preventDefault();
      onChange(restack(report, selectedId, event.key === "]" ? 1 : -1));
    }
  });

  const selected = selectedId ? findBlock(report, selectedId) : null;

  // What the rulers measure: a block coming in from the library, the one being dragged, or the selected one.
  const measured = (): { pageIndex: number; extent: Extent | null } => {
    if (incoming) return { pageIndex: incoming.pageIndex, extent: { rect: incoming.rect, isMoving: true } };
    if (selected && drag.live) return { pageIndex: selected.pageIndex, extent: { rect: drag.live.rect, isMoving: true } };
    if (selected) return { pageIndex: selected.pageIndex, extent: { rect: blockRect(selected.block), isMoving: false } };
    return { pageIndex: currentPage, extent: null };
  };
  const ruler = measured();

  const overlay = (index: number) => {
    const page = pageSize(report.pages[index]);
    if (incoming && incoming.pageIndex === index) return <Measure rect={incoming.rect} page={page} pxPerMm={pxPerMm} isIncoming />;
    if (drag.live && selected?.pageIndex === index) return <Measure rect={drag.live.rect} page={page} pxPerMm={pxPerMm} isIncoming={false} />;
    return textTools(index);
  };

  const textTools = (index: number) => {
    if (!selected || selected.pageIndex !== index || drag.live) return null;
    const block = selected.block;
    if (block.content.kind !== "text") return null;
    const text = block as ReportBlock & { content: { kind: "text"; text: string; style: TextStyle } };
    return (
      <TextBlockTools
        block={text}
        pxPerMm={pxPerMm}
        isEditing={editingId === block.id}
        onStyle={(style, mergeKey) => onChange(updateBlock(report, block.id, { content: { ...text.content, style } }), mergeKey)}
        onSeal={onSeal}
        onText={(words) => onChange(updateBlock(report, block.id, { content: { ...text.content, text: words } }))}
        onEditEnd={() => setEditingId(null)}
        onDelete={remove}
      />
    );
  };

  return (
    <BlockPicturesContext.Provider value={pictures.current}>
      <div className="report-editor">
        <div className="report-column-slot">
          <LibraryPanel
            items={items}
            pictures={library?.pictures ?? {}}
            thumbnails={sheetPictures}
            paper={paper}
            isLoading={library?.isLoading ?? true}
            problems={library?.problems ?? []}
            onAdd={addToCurrent}
            onDrop={drop}
            onDragOver={dragOver}
          />
          <ColumnResizer column={REPORT_LIBRARY_COLUMN} />
        </div>
        <div className="report-main" ref={main}>
          <Rulers desk={main} pageIndex={ruler.pageIndex} page={pageSize(report.pages[ruler.pageIndex])} pxPerMm={pxPerMm} extent={ruler.extent} />
          <PageStack
            report={report}
            pxPerMm={pxPerMm}
            themeId={themeId}
            lookKey={lookKey}
            pageIndex={currentPage}
            selectedId={selectedId}
            editingId={editingId}
            live={drag.live}
            overlay={overlay}
            onChange={onChange}
            onPage={(index) => select(null, index)}
            onBlockPointerDown={drag.beginMove}
            onHandlePointerDown={drag.beginResize}
            onBlockDoubleClick={(block) => block.content.kind === "text" && setEditingId(block.id)}
          />
          <div className="zoom-control">
            <Button icon="zoomOut" tip="Zoom out" shortcut="mod+-" tipSide="above" onClick={() => zoomBy(1 / ZOOM_STEP)} />
            <button type="button" className="zoom-value tabular" data-tip="Fit pages" data-tip-key={shortcutLabel("mod+0")} data-tip-side="above" onClick={fit}>
              {Math.round(zoom * 100)}%
            </button>
            <Button icon="zoomIn" tip="Zoom in" shortcut="mod+=" tipSide="above" onClick={() => zoomBy(ZOOM_STEP)} />
          </div>
        </div>
        <div className="report-column-slot">
          <ColumnResizer column={REPORT_INSPECTOR_COLUMN} />
          <ReportInspector
            report={report}
            catalog={catalog}
            ratios={ratios}
            pageIndex={currentPage}
            selectedId={selectedId}
            onChange={onChange}
            onSeal={onSeal}
            onSelect={(id) => select(id)}
            onAddText={addText}
            onDuplicate={duplicate}
            onRestack={(steps) => selectedId && onChange(restack(report, selectedId, steps))}
            onDelete={remove}
          />
        </div>
      </div>
    </BlockPicturesContext.Provider>
  );
}

function blockRect(block: ReportBlock) {
  return { x: block.x_mm, y: block.y_mm, width: block.width_mm, height: block.height_mm };
}

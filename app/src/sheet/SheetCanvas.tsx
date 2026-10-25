// The sheet in the middle of the workspace: the drawing, the marks over it
// (section line, highlights, eye, pins, text), what shows instead while it
// cannot be drawn, and the zoom pill.

import type { ReactNode, RefObject } from "react";
import type { Look, Pin } from "../lib/types";
import { Canvas, type CanvasControls, type PaperView } from "./Canvas";
import { CanvasMessage } from "./CanvasMessage";
import type { LiveValue } from "./liveValue";
import { MissingData } from "./MissingData";
import { doubleClickPaper, pressPaper, type PaperTools } from "./paperPress";
import { PinOverlay } from "./pins/PinOverlay";
import { SectionOverlay } from "./section/SectionOverlay";
import { TextOverlay } from "./text/TextOverlay";
import type { usePreview } from "./usePreview";
import { ViewPointOverlay } from "./viewpoint/ViewPointOverlay";
import { SheetZoomControl } from "./ZoomControl";

type Props = {
  preview: ReturnType<typeof usePreview>;
  canvas: RefObject<CanvasControls | null>;
  zoom: LiveValue<number>;
  tools: PaperTools;
  highlightOverlay: (view: PaperView) => ReactNode;
  look: Look;
  pins: Pin[];
  diagramName: string;
  onLook: (next: Look) => void;
  onPins: (next: Pin[], mergeKey?: string) => void;
  onSeal: () => void;
  hasScene: boolean;
  catalogError: string | null;
  onRetryCatalog: () => Promise<unknown> | void;
  onFetchAgain: () => void;
  /** The imported model's handles, over the other marks' ground. */
  modelOverlay: (view: PaperView) => ReactNode;
};

export function SheetCanvas(props: Props) {
  const { preview, tools } = props;
  const { section, highlight, viewPoint } = tools;
  const classes = [
    "canvas-wrap",
    section.isDrawing ? "is-drawing-section" : "",
    highlight.isDrawing ? "is-drawing-highlight" : "",
    viewPoint.isPlacing ? "is-placing-view-point" : "",
  ];

  return (
    <div className={classes.join(" ")}>
      <Canvas
        svg={preview.svg}
        controls={props.canvas}
        onZoom={props.zoom.set}
        onPaperPointerDown={(event, view) => pressPaper(event, view, tools)}
        onPaperDoubleClick={(event, view) => doubleClickPaper(event, view, tools)}
        overlay={(view) => preview.svg && <PaperOverlays {...props} svg={preview.svg} view={view} />}
      />
      {!preview.svg && <CanvasMessage catalogError={props.catalogError} previewError={preview.error} onRetry={props.onRetryCatalog} hasScene={props.hasScene} />}
      {preview.svg && <MissingData svg={preview.svg} onFetchAgain={props.onFetchAgain} />}
      {preview.svg && preview.error && (
        <div className="canvas-error" role="alert">
          {preview.error}
        </div>
      )}
      {preview.isSlow && <div className="render-hint" aria-label="Rendering" />}
      <SheetZoomControl zoom={props.zoom} canvas={props.canvas} />
    </div>
  );
}

/** The marks over the drawn sheet, in the order they stack. */
function PaperOverlays({ view, svg, tools, highlightOverlay, modelOverlay, look, pins, diagramName, onLook, onPins, onSeal }: Props & { view: PaperView; svg: string }) {
  const { section, viewPoint, pin, text } = tools;

  return (
    <>
      <SectionOverlay view={view} svg={svg} draft={section.draft} isSelected={section.isSelected} />
      {highlightOverlay(view)}
      <ViewPointOverlay view={view} draft={viewPoint.draft} />
      {modelOverlay(view)}
      <PinOverlay
        view={view}
        svg={svg}
        pins={pins}
        selectedId={pin.selectedId}
        editingId={pin.editingId}
        drag={pin.drag}
        onPins={onPins}
        onSeal={onSeal}
        onEdit={() => pin.setEditingId(pin.selectedId)}
        onEditEnd={() => pin.setEditingId(null)}
        onDeleted={() => pin.setSelectedId(null)}
      />
      <TextOverlay
        view={view}
        svg={svg}
        look={look}
        selectedId={text.selectedId}
        editingId={text.editingId}
        drag={text.drag}
        diagramName={diagramName}
        onLook={onLook}
        onSeal={onSeal}
        onEditEnd={() => text.setEditingId(null)}
        onDeleted={() => text.setSelectedId(null)}
      />
    </>
  );
}

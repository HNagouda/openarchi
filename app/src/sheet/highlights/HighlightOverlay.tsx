// What sits over the highlights: the shape being drawn or dragged, the
// selected one's handles, a bar to restyle it, open it as a sheet or delete
// it, and a field for its label while it is being edited.

import { useLayoutEffect, useState } from "react";
import type { Highlight } from "../../lib/types";
import { Button } from "../../ui/Button";
import type { PaperView } from "../Canvas";
import { LabelField } from "../pins/PinOverlay";
import type { Point } from "../pins/sheetPins";
import { handlesOf, ringOf, withChangedHighlight, withoutHighlight } from "./highlightEdits";
import { rdToPaper } from "./sheetHighlights";
import type { HighlightDraft } from "./useHighlightTool";
import "./highlights.css";

const HANDLE_RADIUS_PX = 5;
// Room between the shape and its bar or label field.
const TOOLBAR_GAP_PX = 10;
const TOOLBAR_HEIGHT_PX = 36;
const LABEL_FIELD_WIDTH_PX = 200;

type Props = {
  view: PaperView;
  svg: string;
  highlights: Highlight[];
  selectedId: string | null;
  editingId: string | null;
  draft: HighlightDraft | null;
  preview: { id: string; shape: Highlight["shape"] } | null;
  onHighlights: (next: Highlight[], mergeKey?: string) => void;
  onSeal: () => void;
  onEdit: () => void;
  onEditEnd: () => void;
  onDeleted: () => void;
  onOpenAsSheet: (highlight: Highlight) => void;
};

export function HighlightOverlay(props: Props) {
  const { view, svg, highlights, selectedId, editingId, draft, preview, onHighlights } = props;
  const [toPaper, setToPaper] = useState<((point: Point) => Point) | null>(null);

  // Read the ground once the SVG is in the page, again whenever the canvas moves or zooms.
  useLayoutEffect(() => {
    const transform = rdToPaper(view.getPaper());
    setToPaper(() => transform);
  }, [svg, view]);

  if (!toPaper) return null;
  const px = view.pxPerMm;
  const onScreen = (point: Point) => {
    const paper = toPaper(point);
    return { x: paper.x * px, y: paper.y * px };
  };
  const pointsOf = (ring: Point[]) => ring.map((point) => onScreen(point)).map(({ x, y }) => `${x},${y}`).join(" ");
  const selected = highlights.find((highlight) => highlight.id === selectedId);
  const shape = preview && preview.id === selectedId ? preview.shape : selected?.shape;
  const ring = shape ? ringOf(shape).map(onScreen) : [];
  const box = boundsOf(ring);

  return (
    <>
      <svg className="highlight-overlay" aria-hidden="true">
        {draft && (draft.isOpen ? <polyline className="highlight-draft" points={pointsOf(draft.ring)} /> : <polygon className="highlight-draft" points={pointsOf(draft.ring)} />)}
        {draft?.isOpen && draft.ring.map((point, index) => <circle key={index} className="highlight-handle" r={HANDLE_RADIUS_PX - 1} cx={onScreen(point).x} cy={onScreen(point).y} />)}
        {preview && <polygon className="highlight-draft" points={pointsOf(ringOf(preview.shape))} />}
        {shape && !draft && <polygon className="highlight-selected" points={ring.map(({ x, y }) => `${x},${y}`).join(" ")} />}
        {shape &&
          !draft &&
          handlesOf(shape).map((point, index) => {
            const at = onScreen(point);
            return <circle key={index} className="highlight-handle" r={HANDLE_RADIUS_PX} cx={at.x} cy={at.y} />;
          })}
      </svg>
      {selected && box && editingId === selected.id && (
        <LabelField
          key={selected.id}
          initial={selected.label}
          style={{ left: box.left, top: box.bottom + TOOLBAR_GAP_PX, width: LABEL_FIELD_WIDTH_PX }}
          onCommit={(label) => {
            // Same key as the drawing, so a new highlight and its label undo together.
            onHighlights(withChangedHighlight(highlights, selected.id, { label }), `highlight:${selected.id}`);
            props.onSeal();
            props.onEditEnd();
          }}
          onCancel={() => {
            props.onSeal();
            props.onEditEnd();
          }}
        />
      )}
      {selected && box && editingId !== selected.id && !preview && !draft && (
        <div className="pin-toolbar-slot" style={{ left: box.left, top: Math.max(0, box.top - TOOLBAR_GAP_PX - TOOLBAR_HEIGHT_PX) }}>
          <HighlightBar highlight={selected} {...props} />
        </div>
      )}
    </>
  );
}

function HighlightBar({ highlight, highlights, onHighlights, onEdit, onDeleted, onOpenAsSheet }: Props & { highlight: Highlight }) {
  const change = (patch: Partial<Omit<Highlight, "id">>) => onHighlights(withChangedHighlight(highlights, highlight.id, patch));

  return (
    <div className="pin-toolbar" role="toolbar" aria-label="Highlight" onPointerDown={(event) => event.stopPropagation()} onDoubleClick={(event) => event.stopPropagation()}>
      <Button icon="lineSolid" tip="Solid outline, for what is there" isActive={highlight.outline === "solid"} onClick={() => change({ outline: "solid" })} />
      <Button icon="lineDashed" tip="Dashed outline, for what is proposed" isActive={highlight.outline === "dashed"} onClick={() => change({ outline: "dashed" })} />
      <span className="toolbar-divider" />
      <Button icon="dim" tip="Dim the rest of the sheet" isActive={highlight.dims_rest} onClick={() => change({ dims_rest: !highlight.dims_rest })} />
      <Button icon="measure" tip="Show the area in m²" isActive={highlight.shows_area} onClick={() => change({ shows_area: !highlight.shows_area })} />
      <span className="toolbar-divider" />
      <Button icon="text" tip="Edit the label" shortcut="enter" onClick={onEdit} />
      <Button icon="openSheet" tip="Open as a sheet of its own" onClick={() => onOpenAsSheet(highlight)} />
      <Button
        icon="trash"
        tip="Delete the highlight"
        shortcut="delete"
        onClick={() => {
          onHighlights(withoutHighlight(highlights, highlight.id));
          onDeleted();
        }}
      />
    </div>
  );
}

/** The screen box around some points, or null for none. */
function boundsOf(points: Point[]): { left: number; top: number; bottom: number } | null {
  if (points.length === 0) return null;
  const xs = points.map((point) => point.x);
  const ys = points.map((point) => point.y);
  return { left: Math.min(...xs), top: Math.min(...ys), bottom: Math.max(...ys) };
}

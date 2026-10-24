// The blocks of the current page in drawing order, the top one first. Drag a
// row to restack it; the arrange buttons and Cmd-[ and Cmd-] do the same
// from the keyboard.

import { useRef, useState, type PointerEvent as ReactPointerEvent } from "react";
import type { BlockContent, Report } from "../lib/types";
import { reorderBlocks } from "./edits";

// A press that moves less than this selects the row instead of dragging it.
const DRAG_START_PX = 3;

type Props = {
  report: Report;
  pageIndex: number;
  selectedId: string | null;
  onChange: (report: Report) => void;
  onSelect: (id: string | null) => void;
};

export function LayerList({ report, pageIndex, selectedId, onChange, onSelect }: Props) {
  const blocks = report.pages[pageIndex].blocks;
  // Shown top first, so the row index runs against the drawing order.
  const rows = [...blocks].reverse();
  const list = useRef<HTMLOListElement>(null);
  const [drag, setDrag] = useState<{ from: number; to: number; startY: number; isMoving: boolean } | null>(null);

  if (rows.length === 0) return <p className="field-note">Drag blocks from the left onto the page.</p>;

  const rowAt = (clientY: number) => {
    const items = [...(list.current?.children ?? [])];
    const index = items.findIndex((item) => clientY < item.getBoundingClientRect().bottom);
    return index === -1 ? items.length - 1 : index;
  };

  const start = (event: ReactPointerEvent, from: number) => {
    if (event.button !== 0) return;
    event.currentTarget.setPointerCapture(event.pointerId);
    setDrag({ from, to: from, startY: event.clientY, isMoving: false });
  };

  const move = (event: ReactPointerEvent) => {
    if (!drag) return;
    const isMoving = drag.isMoving || Math.abs(event.clientY - drag.startY) > DRAG_START_PX;
    setDrag({ ...drag, to: rowAt(event.clientY), isMoving });
  };

  const end = () => {
    if (!drag) return;
    if (!drag.isMoving) onSelect(rows[drag.from].id);
    else if (drag.to !== drag.from) {
      const last = blocks.length - 1;
      onChange(reorderBlocks(report, pageIndex, last - drag.from, last - drag.to));
    }
    setDrag(null);
  };

  return (
    <ol className="report-layers" ref={list} aria-label="Blocks on this page, top first">
      {rows.map((block, index) => {
        const isTarget = drag?.isMoving && drag.to === index && drag.from !== index;
        const classes = ["report-layer", block.id === selectedId ? "is-selected" : "", isTarget ? `is-drop-${drag.to > drag.from ? "below" : "above"}` : ""];
        return (
          <li
            key={block.id}
            className={classes.filter(Boolean).join(" ")}
            onPointerDown={(event) => start(event, index)}
            onPointerMove={move}
            onPointerUp={end}
            onPointerCancel={() => setDrag(null)}
          >
            <span className="report-layer-kind">{kindName(block.content)}</span>
            <span className="report-layer-name">{contentName(block.content)}</span>
          </li>
        );
      })}
    </ol>
  );
}

function kindName(content: BlockContent): string {
  return {
    ratios: "Ratios",
    analysis_figure: "Chart",
    analysis_facts: "Facts",
    analysis_notes: "Notes",
    sheet: "Sheet",
    text: "Text",
  }[content.kind];
}

/** What the block shows, in a few words: "land use", "wind", the first line of a text. */
function contentName(content: BlockContent): string {
  if (content.kind === "ratios") return content.group.replace(/_/g, " ");
  if (content.kind === "sheet") return content.diagram_id.replace(/-/g, " ");
  if (content.kind === "text") return content.text.split("\n")[0];
  return content.section.replace(/-/g, " ");
}

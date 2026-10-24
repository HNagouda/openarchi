// The report's pages as a scrollable stack, each under a small header with
// its number and paper. Drag a header's grip to move the page; the arrows on
// the header do the same from the keyboard.

import { useRef, useState, type PointerEvent as ReactPointerEvent, type ReactNode } from "react";
import type { Report, ReportBlock } from "../lib/types";
import { Button } from "../ui/Button";
import { addPage, deletePage, movePage } from "./edits";
import type { Handle } from "./geometry";
import { PageView, type Live } from "./PageView";

// A press that moves less than this is not a drag.
const DRAG_START_PX = 3;

type Props = {
  report: Report;
  pxPerMm: number;
  themeId: string;
  lookKey: string;
  pageIndex: number;
  selectedId: string | null;
  editingId: string | null;
  live: Live | null;
  /** Drawn over page `index`, such as the text bar of a block on it. */
  overlay: (index: number) => ReactNode;
  onChange: (report: Report) => void;
  onPage: (index: number) => void;
  onBlockPointerDown: (event: ReactPointerEvent, block: ReportBlock, index: number) => void;
  onHandlePointerDown: (event: ReactPointerEvent, block: ReportBlock, handle: Handle, index: number) => void;
  onBlockDoubleClick: (block: ReportBlock) => void;
};

export function PageStack(props: Props) {
  const { report, pageIndex, onChange, onPage } = props;
  const stack = useRef<HTMLDivElement>(null);
  const [drag, setDrag] = useState<{ from: number; to: number; startY: number; isMoving: boolean } | null>(null);

  const slotAt = (clientY: number) => {
    const slots = [...(stack.current?.querySelectorAll(".report-page-slot") ?? [])];
    const index = slots.findIndex((slot) => {
      const box = slot.getBoundingClientRect();
      return clientY < box.top + box.height / 2;
    });
    return index === -1 ? slots.length - 1 : index;
  };

  const start = (event: ReactPointerEvent, from: number) => {
    if (event.button !== 0) return;
    event.currentTarget.setPointerCapture(event.pointerId);
    setDrag({ from, to: from, startY: event.clientY, isMoving: false });
  };

  const move = (event: ReactPointerEvent) => {
    if (!drag) return;
    const isMoving = drag.isMoving || Math.abs(event.clientY - drag.startY) > DRAG_START_PX;
    setDrag({ ...drag, to: slotAt(event.clientY), isMoving });
  };

  const end = () => {
    if (drag?.isMoving && drag.to !== drag.from) {
      onChange(movePage(report, drag.from, drag.to));
      onPage(drag.to);
    }
    setDrag(null);
  };

  const shift = (from: number, to: number) => {
    onChange(movePage(report, from, to));
    onPage(to);
  };

  return (
    <div className="report-stack" ref={stack}>
      {report.pages.map((page, index) => {
        const isTarget = drag !== null && drag.isMoving && drag.to === index && drag.from !== index;
        const classes = ["report-page-slot", index === pageIndex ? "is-current" : "", isTarget ? `is-drop-${drag.to > drag.from ? "below" : "above"}` : ""];
        return (
          <section key={page.id} className={classes.filter(Boolean).join(" ")} aria-label={`Page ${index + 1}`}>
            <header className="report-page-header">
              <span
                className="report-page-grip"
                data-tip="Drag to move the page"
                onPointerDown={(event) => start(event, index)}
                onPointerMove={move}
                onPointerUp={end}
                onPointerCancel={() => setDrag(null)}
              >
                <span className="tabular">{index + 1}</span>
              </span>
              <span className="report-page-paper tabular">
                {page.paper.toUpperCase()} {page.landscape ? "landscape" : "portrait"}
              </span>
              <span className="report-page-actions">
                <Button icon="chevronUp" tip="Move page up" disabled={index === 0} onClick={() => shift(index, index - 1)} />
                <Button icon="chevronDown" tip="Move page down" disabled={index === report.pages.length - 1} onClick={() => shift(index, index + 1)} />
                <Button icon="plus" tip="Add a page after this one" onClick={() => onChange(addPage(report, index))} />
                <Button icon="trash" tip="Delete page" disabled={report.pages.length <= 1} onClick={() => onChange(deletePage(report, index))} />
              </span>
            </header>
            <div className="report-page-wrap">
              <PageView
                report={report}
                index={index}
                pxPerMm={props.pxPerMm}
                themeId={props.themeId}
                lookKey={props.lookKey}
                selectedId={props.selectedId}
                editingId={props.editingId}
                live={props.live}
                onBlockPointerDown={props.onBlockPointerDown}
                onHandlePointerDown={props.onHandlePointerDown}
                onBlockDoubleClick={props.onBlockDoubleClick}
                onPagePointerDown={onPage}
              />
              {props.overlay(index)}
            </div>
          </section>
        );
      })}
      <Button icon="plus" variant="secondary" className="report-add-page" onClick={() => onChange(addPage(report, report.pages.length - 1))}>
        Add page
      </Button>
    </div>
  );
}

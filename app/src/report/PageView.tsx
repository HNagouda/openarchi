// One page of the report: the core's backdrop (background and page number),
// every block over it, and the selected block's outline, handles and the
// snap guides of a drag in progress.

import { useEffect, useState, type PointerEvent as ReactPointerEvent } from "react";
import { errorText, renderReportBackdrop } from "../lib/ipc";
import type { Report, ReportBlock } from "../lib/types";
import { BlockView } from "./BlockView";
import { pageSize } from "./edits";
import { HANDLES, type Guide, type Handle, type Rect } from "./geometry";

/** A drag in progress: the block it moves and where it is now. */
export type Live = { id: string; rect: Rect; guides: Guide[] };

type Props = {
  report: Report;
  index: number;
  pxPerMm: number;
  themeId: string;
  lookKey: string;
  selectedId: string | null;
  editingId: string | null;
  live: Live | null;
  onBlockPointerDown: (event: ReactPointerEvent, block: ReportBlock, index: number) => void;
  onHandlePointerDown: (event: ReactPointerEvent, block: ReportBlock, handle: Handle, index: number) => void;
  onBlockDoubleClick: (block: ReportBlock) => void;
  onPagePointerDown: (index: number) => void;
};

export function PageView(props: Props) {
  const { report, index, pxPerMm, themeId, lookKey, selectedId, editingId, live } = props;
  const page = report.pages[index];
  const size = pageSize(page);
  const backdrop = useBackdrop(report, index, themeId);
  const rectOf = (block: ReportBlock): Rect =>
    live?.id === block.id ? live.rect : { x: block.x_mm, y: block.y_mm, width: block.width_mm, height: block.height_mm };
  const selected = page.blocks.find((block) => block.id === selectedId) ?? null;

  return (
    <div
      className="report-page"
      data-report-page={index}
      data-px-per-mm={pxPerMm}
      style={{ width: size.width * pxPerMm, height: size.height * pxPerMm }}
      onPointerDown={() => props.onPagePointerDown(index)}
    >
      {/* The paper clips what runs past its edge, as the export does; the selection stays whole. */}
      <div className="report-page-content">
        {backdrop && <div className="report-backdrop" dangerouslySetInnerHTML={{ __html: backdrop }} />}
        {page.blocks.map((block) => (
          <BlockView
            key={block.id}
            block={block}
            rect={rectOf(block)}
            pxPerMm={pxPerMm}
            themeId={themeId}
            lookKey={lookKey}
            isSelected={block.id === selectedId}
            isEditing={block.id === editingId}
            onPointerDown={(event, target) => props.onBlockPointerDown(event, target, index)}
            onDoubleClick={props.onBlockDoubleClick}
          />
        ))}
      </div>
      {selected && editingId !== selected.id && (
        <Selection rect={rectOf(selected)} pxPerMm={pxPerMm} onHandle={(event, handle) => props.onHandlePointerDown(event, selected, handle, index)} />
      )}
      {live?.guides.map((guide) => (
        <span
          key={`${guide.axis}${guide.at}`}
          className={`report-guide is-${guide.axis}`}
          style={guide.axis === "x" ? { left: guide.at * pxPerMm } : { top: guide.at * pxPerMm }}
        />
      ))}
    </div>
  );
}

function Selection({ rect, pxPerMm, onHandle }: { rect: Rect; pxPerMm: number; onHandle: (event: ReactPointerEvent, handle: Handle) => void }) {
  return (
    <div
      className="report-selection"
      style={{ left: rect.x * pxPerMm, top: rect.y * pxPerMm, width: rect.width * pxPerMm, height: rect.height * pxPerMm }}
    >
      {HANDLES.map((handle) => (
        <span key={handle} className={`report-handle is-${handle}`} onPointerDown={(event) => onHandle(event, handle)} />
      ))}
    </div>
  );
}

/** The page without its blocks, redrawn when its paper, palette, background or numbering change. */
function useBackdrop(report: Report, index: number, themeId: string): string | null {
  const page = report.pages[index];
  const [svg, setSvg] = useState<string | null>(null);
  const key = JSON.stringify([themeId, report.theme_id, report.background, report.show_page_numbers, report.pages.length, index, page.paper, page.landscape]);

  useEffect(() => {
    let isCancelled = false;
    // Only what the backdrop draws goes over the bridge, not every block.
    const bare: Report = { ...report, pages: report.pages.map((entry) => ({ ...entry, blocks: [] })) };
    renderReportBackdrop(bare, index)
      .then((drawn) => !isCancelled && setSvg(drawn))
      .catch((reason) => console.warn(`The page could not be drawn. ${errorText(reason)}`));
    return () => {
      isCancelled = true;
    };
    // `key` stands for everything in `report` the backdrop draws.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [key]);

  return svg;
}

// Moving a block by dragging it and resizing it by its handles. The block
// follows the pointer on screen only; the report changes once, when the
// pointer lifts, so a whole drag is one undo step. Alt turns snapping off,
// Shift keeps a block's proportions while resizing.

import { useState, type PointerEvent as ReactPointerEvent } from "react";
import { clamp } from "../lib/format";
import type { Report, ReportBlock } from "../lib/types";
import { pageSize, tidy, updateBlock } from "./edits";
import { resize, snapMove, type Handle, type Rect } from "./geometry";
import type { Live } from "./PageView";

// Snapping reaches this far on screen, whatever the zoom.
const SNAP_PX = 6;
// A press that moves less than this is a click that selects, not a drag.
const DRAG_START_PX = 3;

const rectOf = (block: ReportBlock): Rect => ({ x: block.x_mm, y: block.y_mm, width: block.width_mm, height: block.height_mm });

export function useBlockDrag(report: Report, pxPerMm: number, onChange: (report: Report) => void, onSelect: (id: string, pageIndex: number) => void) {
  const [live, setLive] = useState<Live | null>(null);

  const begin = (event: ReactPointerEvent, block: ReportBlock, pageIndex: number, handle: Handle | null) => {
    if (event.button !== 0) return;
    event.stopPropagation();
    onSelect(block.id, pageIndex);
    const startX = event.clientX;
    const startY = event.clientY;
    const start = rectOf(block);
    const page = pageSize(report.pages[pageIndex]);
    const others = report.pages[pageIndex].blocks.filter((other) => other.id !== block.id).map(rectOf);
    let latest: Live | null = null;

    const follow = (move: PointerEvent) => {
      const isPastStart = Math.hypot(move.clientX - startX, move.clientY - startY) >= DRAG_START_PX;
      if (latest === null && !isPastStart) return;
      const dx = (move.clientX - startX) / pxPerMm;
      const dy = (move.clientY - startY) / pxPerMm;
      const threshold = move.altKey ? 0 : SNAP_PX / pxPerMm;
      const result = handle
        ? resize(start, handle, dx, dy, move.shiftKey, page, others, threshold)
        : snapMove(onPage({ ...start, x: start.x + dx, y: start.y + dy }, page), page, others, threshold);
      latest = { id: block.id, rect: result.rect, guides: result.guides };
      setLive(latest);
    };

    const finish = () => {
      window.removeEventListener("pointermove", follow);
      window.removeEventListener("pointerup", finish);
      window.removeEventListener("pointercancel", finish);
      setLive(null);
      if (latest === null) return;
      const { x, y, width, height } = latest.rect;
      onChange(updateBlock(report, block.id, { x_mm: tidy(x), y_mm: tidy(y), width_mm: tidy(width), height_mm: tidy(height) }));
    };

    window.addEventListener("pointermove", follow);
    window.addEventListener("pointerup", finish);
    window.addEventListener("pointercancel", finish);
  };

  return {
    live,
    beginMove: (event: ReactPointerEvent, block: ReportBlock, pageIndex: number) => begin(event, block, pageIndex, null),
    beginResize: (event: ReactPointerEvent, block: ReportBlock, handle: Handle, pageIndex: number) => begin(event, block, pageIndex, handle),
  };
}

/** A moved block stays on its page. */
function onPage(rect: Rect, page: { width: number; height: number }): Rect {
  return { ...rect, x: clamp(rect.x, 0, page.width - rect.width), y: clamp(rect.y, 0, page.height - rect.height) };
}

// One block on its page: the core's picture of it, laid out at its box.
//
// The picture is laid out at the zoomed size rather than scaled with a CSS
// transform, so the vectors stay crisp at every zoom, as on the sheet canvas.

import type { PointerEvent as ReactPointerEvent } from "react";
import type { ReportBlock } from "../lib/types";
import type { Rect } from "./geometry";
import { useBlockSvg } from "./useBlockSvg";

type Props = {
  block: ReportBlock;
  /** Where the block is drawn now: its own box, or where a drag has it. */
  rect: Rect;
  pxPerMm: number;
  themeId: string;
  lookKey: string;
  isSelected: boolean;
  isEditing: boolean;
  onPointerDown: (event: ReactPointerEvent, block: ReportBlock) => void;
  onDoubleClick: (block: ReportBlock) => void;
};

export function BlockView({ block, rect, pxPerMm, themeId, lookKey, isSelected, isEditing, onPointerDown, onDoubleClick }: Props) {
  const { svg, error } = useBlockSvg(themeId, block, lookKey);
  const classes = ["report-block", isSelected ? "is-selected" : "", isEditing ? "is-editing" : "", svg ? "" : "is-loading"];

  return (
    <div
      className={classes.filter(Boolean).join(" ")}
      data-block={block.id}
      style={{ left: rect.x * pxPerMm, top: rect.y * pxPerMm, width: rect.width * pxPerMm, height: rect.height * pxPerMm }}
      onPointerDown={(event) => onPointerDown(event, block)}
      onDoubleClick={() => onDoubleClick(block)}
    >
      {svg && <div className="report-block-picture" dangerouslySetInnerHTML={{ __html: svg }} />}
      {error && <span className="report-block-error">{error}</span>}
    </div>
  );
}

// A block's position and size in millimetres, in a small label under it
// while it is dragged, resized or brought in from the library, and the outline
// of where a block from the library will land.

import type { Rect, Size } from "../geometry";

// Room between the block and its label, and the label's height, so it can go above when there is no room below.
const GAP_PX = 6;
const LABEL_HEIGHT_PX = 19;

type Props = { rect: Rect; page: Size; pxPerMm: number; isIncoming: boolean };

export function Measure({ rect, page, pxPerMm, isIncoming }: Props) {
  const below = (rect.y + rect.height) * pxPerMm + GAP_PX;
  const hasRoomBelow = below + LABEL_HEIGHT_PX <= page.height * pxPerMm;
  const top = hasRoomBelow ? below : rect.y * pxPerMm - GAP_PX - LABEL_HEIGHT_PX;
  const box = { left: rect.x * pxPerMm, top: rect.y * pxPerMm, width: rect.width * pxPerMm, height: rect.height * pxPerMm };

  return (
    <>
      {isIncoming && <div className="report-drop-outline" style={box} />}
      <div className="report-measure tabular" style={{ left: (rect.x + rect.width / 2) * pxPerMm, top }}>
        {millimetres(rect.x)}, {millimetres(rect.y)} · {millimetres(rect.width)} × {millimetres(rect.height)} mm
      </div>
    </>
  );
}

/** 12.345 -> "12.3", 40 -> "40". */
function millimetres(value: number): string {
  return String(Math.round(value * 10) / 10);
}

// The eye while it is dragged, until the core draws it where it was dropped.

import type { PaperView } from "../Canvas";
import type { Point } from "../pins/sheetPins";
import "./viewpoint.css";

const DRAFT_RADIUS_PX = 7;

type Props = {
  view: PaperView;
  draft: Point | null;
};

export function ViewPointOverlay({ view, draft }: Props) {
  if (!draft) return null;
  const px = view.pxPerMm;

  return (
    <svg className="view-point-overlay" aria-hidden="true">
      <circle className="view-point-draft" cx={draft.x * px} cy={draft.y * px} r={DRAFT_RADIUS_PX} />
    </svg>
  );
}

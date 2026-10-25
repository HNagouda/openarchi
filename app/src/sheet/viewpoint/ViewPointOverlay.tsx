// The eye while it is dragged, until the core draws it where it was dropped.

import type { PaperView } from "../Canvas";
import { useLive, type LiveValue } from "../liveValue";
import type { Point } from "../paper";
import "./viewpoint.css";

const DRAFT_RADIUS_PX = 7;

type Props = {
  view: PaperView;
  draft: LiveValue<Point | null>;
};

export function ViewPointOverlay({ view, draft }: Props) {
  const eye = useLive(draft);
  if (!eye) return null;
  const px = view.pxPerMm;

  return (
    <svg className="view-point-overlay" aria-hidden="true">
      <circle className="view-point-draft" cx={eye.x * px} cy={eye.y * px} r={DRAFT_RADIUS_PX} />
    </svg>
  );
}

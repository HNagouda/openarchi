// What sits over the section line: the line being drawn or reshaped, and a
// handle on each end of the selected line to show it can be dragged.

import { useLayoutEffect, useState } from "react";
import type { PaperView } from "../Canvas";
import { useLive, type LiveValue } from "../liveValue";
import { readEnds, type PaperEnds } from "./sectionEdits";
import type { SectionDraft } from "./useSectionTool";
import "./section.css";

const HANDLE_RADIUS_PX = 5;

type Props = {
  view: PaperView;
  svg: string;
  draft: LiveValue<SectionDraft | null>;
  isSelected: boolean;
};

export function SectionOverlay({ view, svg, draft, isSelected }: Props) {
  const [drawn, setDrawn] = useState<PaperEnds | null>(null);

  // Read the marker once the SVG is in the page, again whenever the canvas moves or zooms.
  useLayoutEffect(() => {
    setDrawn(readEnds(view.getPaper()));
  }, [svg, view]);
  const drafted = useLive(draft);

  const ends = drafted ?? (isSelected ? drawn : null);
  if (!ends) return null;
  const px = view.pxPerMm;
  const [x1, y1, x2, y2] = [ends.start.x * px, ends.start.y * px, ends.end.x * px, ends.end.y * px];

  return (
    <svg className="section-overlay" aria-hidden="true">
      {drafted && <line className="section-draft" x1={x1} y1={y1} x2={x2} y2={y2} />}
      <circle className="section-handle" cx={x1} cy={y1} r={HANDLE_RADIUS_PX} />
      <circle className="section-handle" cx={x2} cy={y2} r={HANDLE_RADIUS_PX} />
      <text className="section-handle-label" x={x1} y={y1 - 2 * HANDLE_RADIUS_PX}>
        A
      </text>
      <text className="section-handle-label" x={x2} y={y2 - 2 * HANDLE_RADIUS_PX}>
        A′
      </text>
    </svg>
  );
}

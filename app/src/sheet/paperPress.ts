// Which tool a press on the paper goes to. A line or shape being drawn takes
// every press. Otherwise the View eye comes first, then pins, the section marker and text,
// which all sit on top of the drawing, and highlights last, since a highlight
// covers ground and would otherwise swallow presses meant for what lies on it.
// Each tool lets go of its selection when a press misses it, and the one that
// takes a press clears the selections of the tools it comes before.

import type { PaperView } from "./Canvas";
import type { useHighlightTool } from "./highlights/useHighlightTool";
import type { usePinTool } from "./pins/usePinTool";
import type { useSectionTool } from "./section/useSectionTool";
import type { useTextTool } from "./text/useTextTool";
import type { useViewPointTool } from "./viewpoint/useViewPointTool";

export type PaperTools = {
  section: ReturnType<typeof useSectionTool>;
  highlight: ReturnType<typeof useHighlightTool>;
  pin: ReturnType<typeof usePinTool>;
  text: ReturnType<typeof useTextTool>;
  viewPoint: ReturnType<typeof useViewPointTool>;
};

export function pressPaper(event: React.PointerEvent, view: PaperView, { section, highlight, pin, text, viewPoint }: PaperTools): boolean {
  if (section.isDrawing) return section.onPaperPointerDown(event, view);
  if (highlight.isDrawing) return highlight.onPaperPointerDown(event, view);
  // The eye is placed with one click, or picked up where the core drew it, above everything else.
  if (viewPoint.onPaperPointerDown(event, view)) {
    text.setSelectedId(null);
    section.setSelected(false);
    highlight.setSelectedId(null);
    return true;
  }
  if (pin.onPaperPointerDown(event, view)) {
    text.setSelectedId(null);
    section.setSelected(false);
    highlight.setSelectedId(null);
    return true;
  }
  if (section.onPaperPointerDown(event, view)) {
    text.setSelectedId(null);
    highlight.setSelectedId(null);
    return true;
  }
  if (text.onPaperPointerDown(event, view)) {
    highlight.setSelectedId(null);
    return true;
  }
  return highlight.onPaperPointerDown(event, view);
}

export function doubleClickPaper(event: React.MouseEvent, view: PaperView, { highlight, pin, text }: PaperTools): boolean {
  return highlight.onPaperDoubleClick() || pin.onPaperDoubleClick(event, view) || text.onPaperDoubleClick(event, view);
}

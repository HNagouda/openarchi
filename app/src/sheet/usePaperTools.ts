// The six tools that act on the paper: text, pins, highlights, the section
// line, the View eye and the site edits, all wired to the sheet's undo history.

import { useEffect, type RefObject } from "react";
import { sheetDiagrams } from "../lib/diagrams";
import type { Diagram } from "../lib/types";
import type { CanvasControls } from "./Canvas";
import { useEditTool } from "./edits/useEditTool";
import { useSheetHighlights } from "./highlights/useSheetHighlights";
import type { PaperTools } from "./paperPress";
import { usePinTool } from "./pins/usePinTool";
import { useSectionTool } from "./section/useSectionTool";
import type { useWorkbook } from "./tabs/useWorkbook";
import { useTextTool } from "./text/useTextTool";
import { canPlaceEye } from "./viewpoint/viewPointEdits";
import { useViewPointTool } from "./viewpoint/useViewPointTool";

type Options = {
  history: ReturnType<typeof useWorkbook>;
  diagrams: Diagram[];
  canvas: RefObject<CanvasControls | null>;
  svg: string | null;
  /** False while a dialog or the report editor is over the sheet, or before it can be drawn. */
  isEditable: boolean;
  isReady: boolean;
  toast: (message: string, options?: { isError?: boolean }) => void;
};

export function usePaperTools({ history, diagrams, canvas, svg, isEditable, isReady, toast }: Options) {
  const { book, look } = history;
  const refuse = (message: string) => toast(message);

  const text = useTextTool(look, { change: history.changeLook, seal: history.seal }, canvas, svg, isEditable);
  const pin = usePinTool({ pins: book.pins, onPins: history.changePins, canvas, svg, isEnabled: isEditable, onRefused: refuse });
  const highlights = useSheetHighlights({
    look,
    highlights: book.highlights,
    onHighlights: history.changeHighlights,
    onLook: history.changeLook,
    onSeal: history.seal,
    addAreaSheet: history.addAreaSheet,
    canvas,
    svg,
    isEditable,
    isReady,
    toast,
  });
  const section = useSectionTool({ look, onLook: history.changeLook, svg, isEnabled: isEditable, onRefused: refuse });
  const hasEye = isEditable && canPlaceEye(sheetDiagrams(diagrams, look), look);
  const viewPoint = useViewPointTool({ look, onLook: history.changeLook, svg, isEnabled: hasEye, onRefused: refuse });

  const edit = useEditTool({ edits: book.edits, onEdits: history.changeEdits, look, svg, isEnabled: isEditable, onRefused: refuse });

  // The edit tool takes every press while it is on, so the other marks let go
  // of their selection, or Delete would remove a pin along with a tree.
  useEffect(() => {
    if (!edit.isEditing) return;
    text.setSelectedId(null);
    pin.setSelectedId(null);
    highlights.tool.setSelectedId(null);
    section.setSelected(false);
  }, [edit.isEditing]);

  const tools: PaperTools = { section, highlight: highlights.tool, pin, text, viewPoint, edit };
  return { tools, highlights };
}

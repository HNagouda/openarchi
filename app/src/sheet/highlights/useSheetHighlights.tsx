// Everything the sheet workspace needs for highlights in one place: the tool,
// what it draws over the sheet, its buttons in the tab bar, and "Open as
// sheet", which asks the core for the highlight's shape as a sheet area and
// adds a tab framing it, the same machinery a drawn sheet area uses.

import type { RefObject } from "react";
import { errorText, highlightArea } from "../../lib/ipc";
import type { Highlight, Look } from "../../lib/types";
import type { CanvasControls, PaperView } from "../Canvas";
import { UNLABELLED } from "./highlightEdits";
import { HighlightOverlay } from "./HighlightOverlay";
import { HighlightTools } from "./HighlightTools";
import { useHighlightTool } from "./useHighlightTool";

type Options = {
  look: Look;
  highlights: Highlight[];
  onHighlights: (next: Highlight[], mergeKey?: string) => void;
  onLook: (next: Look) => void;
  onSeal: () => void;
  addAreaSheet: (name: string, selection: NonNullable<Look["selection"]>) => Promise<void>;
  canvas: RefObject<CanvasControls | null>;
  svg: string | null;
  isEditable: boolean;
  isReady: boolean;
  toast: (message: string, options?: { isError?: boolean }) => void;
};

export function useSheetHighlights(options: Options) {
  const { look, highlights, onHighlights, svg, toast } = options;
  const tool = useHighlightTool({
    highlights,
    onHighlights,
    canvas: options.canvas,
    look,
    svg,
    isEnabled: options.isEditable,
    onRefused: (message) => toast(message),
  });

  const openAsSheet = async (highlight: Highlight) => {
    try {
      const area = await highlightArea(highlight);
      await options.addAreaSheet(highlight.label.trim() || UNLABELLED, area);
    } catch (error) {
      toast(`Couldn't open the highlight as a sheet. ${errorText(error)}`, { isError: true });
    }
  };

  const overlay = (view: PaperView) =>
    svg && (
      <HighlightOverlay
        view={view}
        svg={svg}
        highlights={highlights}
        selectedId={tool.selectedId}
        editingId={tool.editingId}
        draft={tool.draft}
        preview={tool.preview}
        onHighlights={onHighlights}
        onSeal={options.onSeal}
        onEdit={() => tool.setEditingId(tool.selectedId)}
        onEditEnd={() => tool.setEditingId(null)}
        onDeleted={() => tool.setSelectedId(null)}
        onOpenAsSheet={openAsSheet}
      />
    );

  const tabTools = (
    <HighlightTools
      look={look}
      isReady={options.isReady}
      isAvailable={tool.isAvailable}
      drawingKind={tool.isDrawing ? tool.kind : null}
      onDraw={tool.startDrawing}
      onStop={tool.stopDrawing}
      onPlace={tool.placeInView}
      onLook={options.onLook}
    />
  );

  return { tool, overlay, tabTools };
}

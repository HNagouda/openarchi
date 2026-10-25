// The sheet workspace's menu commands and single-key shortcuts: undo, export,
// zoom, the two columns, and stepping through diagrams, variants and palettes.

import type { RefObject } from "react";
import { useCommandHandler, type CommandId } from "../lib/commands";
import { usePlainKeys } from "../lib/keys";
import type { CanvasControls } from "./Canvas";
import type { useDiagramStepping } from "./useDiagramStepping";
import type { useSheetColumns } from "./useSheetColumns";

// Every other command this screen answers; undo, redo and export say so as they act.
const HANDLED_COMMANDS: CommandId[] = ["zoom-in", "zoom-out", "zoom-fit", "inspector", "diagrams", "home", "add-text"];

type Options = {
  canvas: RefObject<CanvasControls | null>;
  columns: ReturnType<typeof useSheetColumns>;
  stepping: ReturnType<typeof useDiagramStepping>;
  isReady: boolean;
  /** False while a dialog or the report editor is over the sheet. */
  isEnabled: boolean;
  /** True when no text, pin or highlight is selected, so the arrows are free to step diagrams. */
  isNothingSelected: boolean;
  onUndo: () => void;
  onRedo: () => void;
  onExport: () => void;
  onAddText: () => void;
  onLeave: () => void;
};

export function useSheetKeys({ canvas, columns, stepping, isReady, isEnabled, isNothingSelected, onUndo, onRedo, onExport, onAddText, onLeave }: Options) {
  useCommandHandler((id) => {
    if (id === "undo") {
      onUndo();
      return true;
    }
    if (id === "redo") {
      onRedo();
      return true;
    }
    if (id === "export") {
      onExport();
      return true;
    }
    if (id === "zoom-in") canvas.current?.zoomIn();
    if (id === "zoom-out") canvas.current?.zoomOut();
    if (id === "zoom-fit") canvas.current?.fit();
    if (id === "inspector") columns.toggleInspector();
    if (id === "diagrams") columns.toggleDiagrams();
    if (id === "add-text" && isReady) onAddText();
    if (id === "home") onLeave();
    return HANDLED_COMMANDS.includes(id);
  });

  usePlainKeys((key, event) => {
    if (key === "[") stepping.stepTheme(-1);
    if (key === "]") stepping.stepTheme(1);
    if (key.toLowerCase() === "i" && !event.repeat) columns.toggleInspector();
    if (key.toLowerCase() === "d" && !event.repeat) columns.toggleDiagrams();
    // With a text box selected the arrows nudge it instead; see useTextTool.
    const steps: Record<string, () => void> = {
      ArrowUp: () => stepping.stepDiagram(-1),
      ArrowDown: () => stepping.stepDiagram(1),
      ArrowLeft: () => stepping.stepVariant(-1),
      ArrowRight: () => stepping.stepVariant(1),
    };
    if (steps[key] && isNothingSelected) {
      event.preventDefault();
      steps[key]();
    }
  }, isEnabled);
}

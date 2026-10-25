// Whether the diagram column and the inspector are open, remembered between
// sessions, and the toast kept centred on the sheet between them.

import { useEffect, useState } from "react";
import { readPreference, writePreference } from "../lib/storage";

export function useSheetColumns() {
  const [isInspectorOpen, setInspectorOpen] = useState(() => readPreference("inspectorOpen", true));
  const [isDiagramsOpen, setDiagramsOpen] = useState(() => readPreference("diagramsOpen", true));

  const toggleInspector = () => {
    setInspectorOpen(!isInspectorOpen);
    writePreference("inspectorOpen", !isInspectorOpen);
  };

  const toggleDiagrams = () => {
    setDiagramsOpen(!isDiagramsOpen);
    writePreference("diagramsOpen", !isDiagramsOpen);
  };

  // Toasts sit centred on the sheet, between the two columns. A narrow window
  // lays the diagram column over the sheet, so the toast does not move for it there.
  useEffect(() => {
    const root = document.documentElement.style;
    root.setProperty("--toast-right", isInspectorOpen ? "var(--inspector-width)" : "0px");
    root.setProperty("--toast-left", isDiagramsOpen ? "var(--diagram-column-pushes)" : "0px");
    return () => {
      root.removeProperty("--toast-right");
      root.removeProperty("--toast-left");
    };
  }, [isInspectorOpen, isDiagramsOpen]);

  return { isInspectorOpen, isDiagramsOpen, toggleInspector, toggleDiagrams };
}

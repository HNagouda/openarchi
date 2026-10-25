// Moving through the catalog from the sheet: picking a diagram or a variant in
// the column, and stepping to the next diagram, variant or palette from the keyboard.

import { useCallback, useRef } from "react";
import { chosenVariant } from "../lib/diagrams";
import type { Diagram, Look, Theme } from "../lib/types";

type Direction = 1 | -1;

export function useDiagramStepping(look: Look, diagrams: Diagram[], themes: Theme[], changeLook: (next: Look) => void) {
  // Read through a ref, so the diagram column's callbacks stay the same while the look changes.
  const latestLook = useRef(look);
  latestLook.current = look;

  // A new diagram starts on its first variant, and a plan-only one turns an axonometric sheet to its plan in the same step.
  // Explode plates are flat maps, so an explode sheet stays.
  const selectDiagram = useCallback(
    (diagram_id: string) => {
      const current = latestLook.current;
      if (diagram_id === current.diagram_id) return;
      const isPlanOnly = diagrams.find((entry) => entry.id === diagram_id)?.plan_only === true;
      const isAxonometric = current.view.mode === "axonometric";
      const view = isPlanOnly && isAxonometric ? { ...current.view, mode: "plan" as const } : current.view;
      changeLook({ ...current, diagram_id, variant: null, view });
    },
    [diagrams, changeLook],
  );

  const selectVariant = useCallback((variantId: string) => changeLook({ ...latestLook.current, variant: variantId }), [changeLook]);

  const stepDiagram = (direction: Direction) => {
    const index = diagrams.findIndex((entry) => entry.id === look.diagram_id);
    const next = diagrams[index + direction];
    if (next) selectDiagram(next.id);
  };

  const stepVariant = (direction: Direction) => {
    const diagram = diagrams.find((entry) => entry.id === look.diagram_id);
    const variant = diagram ? chosenVariant(diagram, look.variant) : null;
    if (!diagram || !variant) return;
    const index = diagram.variants.indexOf(variant);
    const next = diagram.variants[index + direction];
    if (next) selectVariant(next.id);
  };

  const stepTheme = (direction: Direction) => {
    if (themes.length === 0) return;
    const index = themes.findIndex((theme) => theme.id === look.theme_id);
    const next = themes[(index + direction + themes.length) % themes.length];
    changeLook({ ...look, theme_id: next.id });
  };

  return { selectDiagram, selectVariant, stepDiagram, stepVariant, stepTheme };
}

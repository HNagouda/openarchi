// Diagrams as the app shows them: in their groups, under the name of the
// chosen variant, and as explode plates. The core resolves the same ids in
// crates/core/src/diagram.rs and catalog/lookup.rs.

import type { Diagram, Variant } from "./types";

/** Splits a plate id that names one variant, as the core does: "air-quality:pm10". */
const VARIANT_SEPARATOR = ":";

type DiagramGroup = { name: string; diagrams: Diagram[] };

/** The variant a look draws: the one it names, or the diagram's first. Null without variants. */
export function chosenVariant(diagram: Diagram, variantId: string | null | undefined): Variant | null {
  return diagram.variants.find((variant) => variant.id === variantId) ?? diagram.variants[0] ?? null;
}

/** The sheet title for a variant, "Air quality, PM10", or the diagram's own name. */
export function drawnName(diagram: Diagram, variantId: string | null | undefined): string {
  return chosenVariant(diagram, variantId)?.title ?? diagram.name;
}

/** The diagram with its variant's layers and focus filled in, as the core draws it. */
export function drawnDiagram(diagram: Diagram, variantId: string | null | undefined): Diagram {
  const variant = chosenVariant(diagram, variantId);
  if (!variant) return diagram;
  return { ...diagram, name: variant.title, description: variant.description, layers: variant.layers, emphasis: variant.emphasis, variants: [] };
}

/** Diagrams in catalog order under their group headings. The core keeps each group together. */
export function groupDiagrams(diagrams: Diagram[]): DiagramGroup[] {
  const groups: DiagramGroup[] = [];
  for (const diagram of diagrams) {
    const last = groups[groups.length - 1];
    if (last && last.name === diagram.group) last.diagrams.push(diagram);
    else groups.push({ name: diagram.group, diagrams: [diagram] });
  }

  return groups;
}

/** "air-quality" and "pm10" -> "air-quality:pm10"; no variant -> the diagram id. */
function plateId(diagramId: string, variantId?: string | null): string {
  return variantId ? `${diagramId}${VARIANT_SEPARATOR}${variantId}` : diagramId;
}

/** Every plate the explode view can stack: each diagram, or each variant of one that has them. A plan-only diagram is no plate. */
export function plateOptions(diagrams: Diagram[]): { id: string; name: string }[] {
  return diagrams.filter((diagram) => !diagram.plan_only).flatMap((diagram) =>
    diagram.variants.length === 0
      ? [{ id: diagram.id, name: diagram.name }]
      : diagram.variants.map((variant) => ({ id: plateId(diagram.id, variant.id), name: variant.title })),
  );
}

/** A plate's name on screen; an id the catalog no longer has shows as it is. */
export function plateName(diagrams: Diagram[], plate: string): string {
  const [diagramId, variantId] = plate.split(VARIANT_SEPARATOR);
  const diagram = diagrams.find((entry) => entry.id === diagramId);
  return diagram ? drawnName(diagram, variantId) : plate;
}

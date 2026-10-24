// The explode view's plates on the rendered sheet, read from the SVG the core drew.
//
// The core writes each plate as a group with its plate id, its place in the
// stack, the paper y of its centre and its left and right edge in data
// attributes (crates/core/src/render/explode.rs), so the app never repeats
// the stack's layout. The SVG's user unit is the millimetre.

export type SheetPlate = {
  id: string;
  /** 0 for the top plate. */
  index: number;
  centreY: number;
  left: number;
  right: number;
};

/** Every plate on the sheet, top first. */
export function readPlates(paper: Element | null): SheetPlate[] {
  const groups = paper ? [...paper.querySelectorAll<SVGGElement>("g[data-plate]")] : [];
  const plates = groups.map((group) => ({
    id: group.dataset.plate ?? "",
    index: Number(group.dataset.index),
    centreY: Number(group.dataset.centreY),
    left: Number(group.dataset.left),
    right: Number(group.dataset.right),
  }));
  return plates.sort((a, b) => a.index - b.index);
}

/** The plate group drawn at a screen point. Plates overlap, so this asks the browser which one shows there. */
export function plateGroupAt(clientX: number, clientY: number): SVGGElement | null {
  return document.elementFromPoint(clientX, clientY)?.closest<SVGGElement>("g[data-plate]") ?? null;
}

/** Where a plate dragged from `from` lands when its centre is at `y`: below every other plate whose centre it has passed. */
export function dropIndex(plates: SheetPlate[], from: number, y: number): number {
  return plates.filter((plate) => plate.index !== from && plate.centreY < y).length;
}

// The pins on the rendered sheet, read from the SVG the core drew.
//
// The core writes each pin as a group with its id, icon, label and paper
// anchor in data attributes, as it does for sheet text.

import type { PinIcon } from "../../lib/types";
import type { Box } from "../paper";

export type SheetPin = {
  id: string;
  icon: PinIcon;
  label: string;
  /** The anchor in paper millimetres: the point the marker stands on. */
  x: number;
  y: number;
  box: Box;
};

export function readPins(paper: Element | null): SheetPin[] {
  const groups = paper ? [...paper.querySelectorAll<SVGGElement>("g[data-pin]")] : [];
  return groups.map((group) => {
    const data = group.dataset;
    const bbox = group.getBBox();
    return {
      id: data.pin ?? "",
      icon: (data.icon as PinIcon | undefined) ?? "pin",
      label: data.label ?? "",
      x: Number(data.x),
      y: Number(data.y),
      box: { x: bbox.x, y: bbox.y, width: bbox.width, height: bbox.height },
    };
  });
}

export function pinGroup(paper: Element | null, id: string): SVGGElement | null {
  return paper?.querySelector<SVGGElement>(`g[data-pin="${CSS.escape(id)}"]`) ?? null;
}

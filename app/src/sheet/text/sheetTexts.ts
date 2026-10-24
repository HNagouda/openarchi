// The texts on the rendered sheet, read from the SVG the core drew.
//
// The core writes each text as a group with its id, anchor and resolved style
// in data attributes, so the app never guesses where the layout put the title
// or which size "auto" came to. The SVG's user unit is the millimetre, so
// bounding boxes come back in paper millimetres.

import type { Align } from "../../lib/types";

export type Box = { x: number; y: number; width: number; height: number };

export type SheetText = {
  id: string;
  /** The anchor in millimetres: the top of the first line, at the aligned edge. */
  x: number;
  y: number;
  box: Box;
  align: Align;
  family: string;
  sizePt: number;
  isBold: boolean;
  isItalic: boolean;
  /** "#RRGGBB" as drawn. */
  colour: string;
  opacity: number;
};

// Small texts are hard to hit exactly; this much paper around each one counts too.
const HIT_SLOP_MM = 1.5;

export function readTexts(paper: Element | null): SheetText[] {
  const groups = paper ? [...paper.querySelectorAll<SVGGElement>("g[data-text]")] : [];
  return groups.map(readText);
}

/** The text under a point on the paper, the one drawn last when several overlap. */
export function textAt(texts: SheetText[], x: number, y: number): SheetText | null {
  const hits = texts.filter(({ box }) => {
    const isInsideX = x >= box.x - HIT_SLOP_MM && x <= box.x + box.width + HIT_SLOP_MM;
    const isInsideY = y >= box.y - HIT_SLOP_MM && y <= box.y + box.height + HIT_SLOP_MM;
    return isInsideX && isInsideY;
  });
  return hits[hits.length - 1] ?? null;
}

export function groupFor(paper: Element | null, id: string): SVGGElement | null {
  return paper?.querySelector<SVGGElement>(`g[data-text="${CSS.escape(id)}"]`) ?? null;
}

function readText(group: SVGGElement): SheetText {
  const data = group.dataset;
  const bbox = group.getBBox();
  return {
    id: data.text ?? "",
    x: Number(data.x),
    y: Number(data.y),
    box: { x: bbox.x, y: bbox.y, width: bbox.width, height: bbox.height },
    align: (data.align as Align | undefined) ?? "left",
    family: data.family ?? "Inter",
    sizePt: Number(data.sizePt),
    isBold: data.bold === "true",
    isItalic: data.italic === "true",
    colour: data.colour ?? "#000000",
    opacity: Number(data.opacity ?? 1),
  };
}

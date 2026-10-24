// The two kinds of text the report inspector adds, and where a new one goes:
// across the margins under the lowest block, as the next line of a document.

import type { BlockContent, ReportPage, TextStyle } from "../lib/types";
import { pageSize } from "./edits";
import { PAGE_MARGIN_MM, type Rect } from "./geometry";

export type TextKind = "heading" | "body" | "caption";

// Every kind of text the inspector offers, in one place.
const PRESETS: Record<TextKind, { text: string; style: TextStyle; heightMm: number }> = {
  heading: { text: "Heading", style: { size_pt: 24, bold: true }, heightMm: 14 },
  body: { text: "Write about the site here.", style: { size_pt: 10 }, heightMm: 30 },
  caption: { text: "Caption", style: { size_pt: 7, italic: true }, heightMm: 8 },
};
// Room left between the lowest block and the new text under it.
const GAP_MM = 6;

export function textContent(kind: TextKind): BlockContent {
  return { kind: "text", text: PRESETS[kind].text, style: PRESETS[kind].style };
}

/** The box a new text gets: between the margins under the lowest block, or in the middle when it no longer fits. */
export function textBox(page: ReportPage, kind: TextKind): Rect {
  const size = pageSize(page);
  const width = size.width - 2 * PAGE_MARGIN_MM;
  const height = PRESETS[kind].heightMm;
  const bottoms = page.blocks.map((block) => block.y_mm + block.height_mm);
  const y = bottoms.length === 0 ? PAGE_MARGIN_MM : Math.max(PAGE_MARGIN_MM, ...bottoms) + GAP_MM;

  if (y + height <= size.height - PAGE_MARGIN_MM) return { x: PAGE_MARGIN_MM, y, width, height };
  return { x: PAGE_MARGIN_MM, y: (size.height - height) / 2, width, height };
}

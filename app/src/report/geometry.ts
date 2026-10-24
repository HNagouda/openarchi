// Moving and resizing a block on its page, in millimetres, with snapping to
// the page edges, the margins, the centre lines and the other blocks.

import { MIN_BLOCK_MM } from "./edits";

export type Rect = { x: number; y: number; width: number; height: number };
export type Size = { width: number; height: number };
/** A line a block snapped to, drawn while dragging: x = at for "x", y = at for "y". */
export type Guide = { axis: "x" | "y"; at: number };
export type Handle = "n" | "s" | "e" | "w" | "ne" | "nw" | "se" | "sw";

// Every handle a selected block shows, in one place.
export const HANDLES: Handle[] = ["nw", "n", "ne", "e", "se", "s", "sw", "w"];
/** The margin a block snaps to inside each page edge. */
export const PAGE_MARGIN_MM = 15;

type Lines = { xs: number[]; ys: number[] };

/** Every line a block can snap to on this page. */
function snapLines(page: Size, others: Rect[]): Lines {
  const xs = [0, PAGE_MARGIN_MM, page.width / 2, page.width - PAGE_MARGIN_MM, page.width];
  const ys = [0, PAGE_MARGIN_MM, page.height / 2, page.height - PAGE_MARGIN_MM, page.height];
  for (const other of others) {
    xs.push(other.x, other.x + other.width / 2, other.x + other.width);
    ys.push(other.y, other.y + other.height / 2, other.y + other.height);
  }
  return { xs, ys };
}

/** The shift that brings the nearest of `edges` onto one of `lines`, if any is within `threshold`. */
function nearest(edges: number[], lines: number[], threshold: number): { shift: number; line: number } | null {
  let best: { shift: number; line: number } | null = null;
  for (const edge of edges) {
    for (const line of lines) {
      const shift = line - edge;
      if (Math.abs(shift) <= threshold && (best === null || Math.abs(shift) < Math.abs(best.shift))) {
        best = { shift, line };
      }
    }
  }
  return best;
}

/** `rect` with its left, centre or right and its top, centre or bottom snapped. */
export function snapMove(rect: Rect, page: Size, others: Rect[], threshold: number): { rect: Rect; guides: Guide[] } {
  const lines = snapLines(page, others);
  const guides: Guide[] = [];
  let { x, y } = rect;
  const alongX = nearest([x, x + rect.width / 2, x + rect.width], lines.xs, threshold);
  if (alongX) {
    x += alongX.shift;
    guides.push({ axis: "x", at: alongX.line });
  }
  const alongY = nearest([y, y + rect.height / 2, y + rect.height], lines.ys, threshold);
  if (alongY) {
    y += alongY.shift;
    guides.push({ axis: "y", at: alongY.line });
  }
  return { rect: { ...rect, x, y }, guides };
}

/**
 * `start` with the edges `handle` holds moved by (`dx`, `dy`), the moving
 * edges snapped, never smaller than the smallest block. With `keepsRatio`
 * the block keeps the proportions it had when the drag began.
 */
export function resize(
  start: Rect,
  handle: Handle,
  dx: number,
  dy: number,
  keepsRatio: boolean,
  page: Size,
  others: Rect[],
  threshold: number,
): { rect: Rect; guides: Guide[] } {
  const lines = snapLines(page, others);
  const guides: Guide[] = [];
  let left = start.x;
  let right = start.x + start.width;
  let top = start.y;
  let bottom = start.y + start.height;

  const snapEdge = (value: number, axis: "x" | "y") => {
    const hit = nearest([value], axis === "x" ? lines.xs : lines.ys, threshold);
    if (!hit) return value;
    guides.push({ axis, at: hit.line });
    return hit.line;
  };
  if (handle.includes("w")) left = Math.min(snapEdge(left + dx, "x"), right - MIN_BLOCK_MM);
  if (handle.includes("e")) right = Math.max(snapEdge(right + dx, "x"), left + MIN_BLOCK_MM);
  if (handle.includes("n")) top = Math.min(snapEdge(top + dy, "y"), bottom - MIN_BLOCK_MM);
  if (handle.includes("s")) bottom = Math.max(snapEdge(bottom + dy, "y"), top + MIN_BLOCK_MM);

  const rect = { x: left, y: top, width: right - left, height: bottom - top };
  if (!keepsRatio) return { rect, guides };
  // Snapped edges would fight the ratio, so a proportional resize shows no guides.
  return { rect: proportional(start, rect, handle), guides: [] };
}

/** `rect` turned back to `start`'s proportions, anchored at the edges the handle does not hold. */
function proportional(start: Rect, rect: Rect, handle: Handle): Rect {
  const ratio = start.width / start.height;
  const isWidthLed = handle === "e" || handle === "w" || (handle.length === 2 && rect.width / start.width >= rect.height / start.height);
  const width = isWidthLed ? rect.width : rect.height * ratio;
  const height = isWidthLed ? rect.width / ratio : rect.height;
  const x = handle.includes("w") ? start.x + start.width - width : start.x;
  const y = handle.includes("n") ? start.y + start.height - height : start.y;
  return { x, y, width, height };
}

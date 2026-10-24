// The inner edge of a side column, dragged to make the column wider or
// narrower. Double-click puts it back to its usual width; with the edge
// focused, the arrow keys move it too. The width is kept per machine and
// set as the column's CSS variable on the root, so everything that already
// reads that variable (the column, the toasts) follows without being told.

import { useLayoutEffect, useState } from "react";
import { clamp } from "../../lib/format";
import { readPreference, writePreference } from "../../lib/storage";
import "./columns.css";

const KEY_STEP_PX = 16;
const KEY_STEP_LARGE_PX = 64;
// Neither column takes more than this share of the window, so the sheet or the report pages keep at least 40 %.
const MAX_WINDOW_SHARE = 0.3;
// Turns off the slide-open animation while dragging, so the column keeps up with the pointer.
const RESIZING_CLASS = "is-resizing-columns";

export type Column = {
  label: string;
  cssVariable: string;
  preference: string;
  min: number;
  max: number;
  /** Which side of the column the handle sits on: the side facing the sheet. */
  edge: "left" | "right";
};

// Every resizable column, in one place. The usual widths live in tokens.css.
export const DIAGRAM_COLUMN: Column = {
  label: "Diagram column width",
  cssVariable: "--diagram-column-width",
  preference: "diagramColumnWidth",
  min: 200,
  max: 440,
  edge: "right",
};

export const INSPECTOR_COLUMN: Column = {
  label: "Inspector width",
  cssVariable: "--inspector-width",
  preference: "inspectorWidth",
  min: 280,
  max: 720,
  edge: "left",
};

export const REPORT_LIBRARY_COLUMN: Column = {
  label: "Blocks column width",
  cssVariable: "--report-library-width",
  preference: "reportLibraryWidth",
  min: 200,
  max: 440,
  edge: "right",
};

export const REPORT_INSPECTOR_COLUMN: Column = {
  label: "Report inspector width",
  cssVariable: "--report-inspector-width",
  preference: "reportInspectorWidth",
  min: 280,
  max: 720,
  edge: "left",
};

export function ColumnResizer({ column }: { column: Column }) {
  const [width, setWidth] = useState<number | null>(() => readPreference<number | null>(column.preference, null));

  // Set while the column is on screen and taken off after, so the site step keeps its own panel width.
  useLayoutEffect(() => {
    if (width === null) return;
    const root = document.documentElement.style;
    root.setProperty(column.cssVariable, `${width}px`);
    return () => {
      root.removeProperty(column.cssVariable);
    };
  }, [width, column.cssVariable]);

  const change = (next: number | null) => {
    const kept = next === null ? null : clamp(Math.round(next), column.min, maxWidth(column));
    setWidth(kept);
    writePreference(column.preference, kept);
  };

  const startDrag = (event: React.PointerEvent) => {
    if (event.button !== 0) return;
    event.preventDefault();
    const startX = event.clientX;
    const startWidth = currentWidth(column);
    // Dragging the left edge to the left makes the column wider.
    const direction = column.edge === "right" ? 1 : -1;
    document.body.classList.add(RESIZING_CLASS);

    const onMove = (move: PointerEvent) => {
      const next = startWidth + (move.clientX - startX) * direction;
      setWidth(clamp(Math.round(next), column.min, maxWidth(column)));
    };
    const onUp = (up: PointerEvent) => {
      window.removeEventListener("pointermove", onMove);
      window.removeEventListener("pointerup", onUp);
      document.body.classList.remove(RESIZING_CLASS);
      change(startWidth + (up.clientX - startX) * direction);
    };
    window.addEventListener("pointermove", onMove);
    window.addEventListener("pointerup", onUp);
  };

  const onKeyDown = (event: React.KeyboardEvent) => {
    if (event.key !== "ArrowLeft" && event.key !== "ArrowRight") return;
    // The sheet uses the arrows to step through diagrams; here they move the edge instead.
    event.preventDefault();
    event.stopPropagation();
    const step = event.shiftKey ? KEY_STEP_LARGE_PX : KEY_STEP_PX;
    const outward = (event.key === "ArrowRight") === (column.edge === "right");
    change(currentWidth(column) + (outward ? step : -step));
  };

  return (
    <div
      className={`column-resizer is-${column.edge}`}
      role="separator"
      aria-orientation="vertical"
      aria-label={column.label}
      aria-valuemin={column.min}
      aria-valuemax={column.max}
      aria-valuenow={width ?? undefined}
      tabIndex={0}
      data-tip="Drag to resize, double-click to reset"
      onPointerDown={startDrag}
      onDoubleClick={() => change(null)}
      onKeyDown={onKeyDown}
    />
  );
}

/** The column's width as drawn now, from the variable whether the user set it or tokens.css did. */
function currentWidth(column: Column): number {
  return parseFloat(getComputedStyle(document.documentElement).getPropertyValue(column.cssVariable)) || column.min;
}

function maxWidth(column: Column): number {
  return Math.max(column.min, Math.min(column.max, window.innerWidth * MAX_WINDOW_SHARE));
}

// Drawing, selecting, reshaping and deleting a sheet's section line.
//
// With the tool on, a drag draws the line, or two clicks do, the line
// following the pointer in between. Without it, the core's marker can be
// picked up by either end and dragged. Like pins, only the drop changes the
// look, so every edit is one undo step, and paper points go through the
// sheet's ground points into RD New metres, so a line drawn on a tilted view
// lands on the same ground in every other view.

import { useEffect, useRef, useState } from "react";
import { usePlainKeys } from "../../lib/keys";
import type { Look } from "../../lib/types";
import type { PaperView } from "../Canvas";
import { useLiveValue } from "../liveValue";
import { followPointer } from "../pointerDrag";
import { paperToRd, type Point } from "../paper";
import { distanceToSegment, MIN_LENGTH_M, readEnds, withoutSection, withSection } from "./sectionEdits";

// How close to an end or the line, in paper millimetres, a press still picks it up.
const END_HIT_MM = 3;
const LINE_HIT_MM = 1.5;
export const SECTION_KEY = "s";

/** The line as it is being drawn or reshaped, in paper millimetres. */
export type SectionDraft = { start: Point; end: Point };

type Options = {
  look: Look;
  onLook: (next: Look) => void;
  svg: string | null;
  isEnabled: boolean;
  /** Says why a line could not be drawn where it was. */
  onRefused: (message: string) => void;
};

export function useSectionTool({ look, onLook, svg, isEnabled, onRefused }: Options) {
  const [isDrawing, setDrawing] = useState(false);
  const [isSelected, setSelected] = useState(false);
  const draft = useLiveValue<SectionDraft | null>(null);
  // After a first click, the press that will finish the line, and how to stop following the pointer.
  const pending = useRef<{ start: Point; stop: () => void } | null>(null);
  const latestLook = useRef(look);
  latestLook.current = look;
  const isSectionView = look.view.mode === "section";
  const hasLine = Boolean(look.section);

  // The core's new SVG has the line where the drop put it.
  useEffect(() => draft.set(null), [draft, svg]);
  // A line that undo took away is no longer selected; the section view has no line to draw on.
  useEffect(() => {
    if (!hasLine) setSelected(false);
  }, [hasLine]);
  useEffect(() => {
    if (isSectionView || !isEnabled) stopDrawing();
  }, [isSectionView, isEnabled]);

  function stopDrawing() {
    pending.current?.stop();
    pending.current = null;
    draft.set(null);
    setDrawing(false);
  }

  /** Saves a line between two RD New points, or says why not. */
  const commit = (start: Point, end: Point): boolean => {
    if (Math.hypot(end.x - start.x, end.y - start.y) < MIN_LENGTH_M) {
      onRefused(`A section line needs to be at least ${MIN_LENGTH_M} m long.`);
      return false;
    }
    onLook(withSection(latestLook.current, start, end));
    setSelected(true);
    return true;
  };

  /** RD New metres for paper points, or null with the reason told when this view has no ground. */
  const toGround = (paper: Element | null) => {
    const toRd = paperToRd(paper);
    if (!toRd) onRefused("Section lines are drawn on a plan or a tilted view.");
    return toRd;
  };

  const finish = (paper: Element | null, start: Point, end: Point) => {
    const toRd = toGround(paper);
    const isSaved = toRd !== null && commit(toRd(start), toRd(end));
    if (isSaved) stopDrawing();
    else draft.set(null);
  };

  const startDrawing = (event: React.PointerEvent, view: PaperView) => {
    const paper = view.getPaper();
    const point = view.toMm(event.clientX, event.clientY);
    if (pending.current) {
      const { start } = pending.current;
      pending.current.stop();
      pending.current = null;
      finish(paper, start, point);
      return;
    }

    draft.set({ start: point, end: point });
    const follow = (move: PointerEvent) => draft.set({ start: point, end: view.toMm(move.clientX, move.clientY) });
    followPointer(event, {
      onDrag: follow,
      onDrop: (up, isDragged) => {
        if (isDragged) {
          finish(paper, point, view.toMm(up.clientX, up.clientY));
          return;
        }
        // A click: the line follows the pointer until the second click.
        window.addEventListener("pointermove", follow);
        pending.current = { start: point, stop: () => window.removeEventListener("pointermove", follow) };
      },
    });
  };

  /** Drags one end; the other keeps its exact place on the ground. */
  const dragEnd = (which: "start" | "end", event: React.PointerEvent, view: PaperView) => {
    const paper = view.getPaper();
    const ends = readEnds(paper);
    const line = latestLook.current.section;
    if (!ends || !line) return;
    followPointer(event, {
      onDrag: (move) => {
        const point = view.toMm(move.clientX, move.clientY);
        draft.set(which === "start" ? { start: point, end: ends.end } : { start: ends.start, end: point });
      },
      onDrop: (up, isDragged) => {
        if (!isDragged) return;
        const toRd = toGround(paper);
        const moved = toRd?.(view.toMm(up.clientX, up.clientY));
        const [startRd, endRd] = [{ x: line.start[0], y: line.start[1] }, { x: line.end[0], y: line.end[1] }];
        const isSaved = moved !== undefined && commit(which === "start" ? moved : startRd, which === "end" ? moved : endRd);
        if (!isSaved) draft.set(null);
      },
    });
  };

  const onPaperPointerDown = (event: React.PointerEvent, view: PaperView): boolean => {
    if (isDrawing) {
      event.preventDefault();
      startDrawing(event, view);
      return true;
    }
    const ends = readEnds(view.getPaper());
    const point = view.toMm(event.clientX, event.clientY);
    if (!ends) {
      setSelected(false);
      return false;
    }
    const isNear = (end: Point) => Math.hypot(point.x - end.x, point.y - end.y) <= END_HIT_MM;
    if (isNear(ends.start) || isNear(ends.end)) {
      event.preventDefault();
      setSelected(true);
      dragEnd(isNear(ends.start) ? "start" : "end", event, view);
      return true;
    }
    if (distanceToSegment(point, ends.start, ends.end) <= LINE_HIT_MM) {
      event.preventDefault();
      setSelected(true);
      return true;
    }
    setSelected(false);
    return false;
  };

  const toggle = () => {
    if (isDrawing) {
      stopDrawing();
      return;
    }
    if (isSectionView) return;
    setSelected(false);
    setDrawing(true);
  };

  const remove = () => {
    onLook(withoutSection(latestLook.current));
    setSelected(false);
  };

  usePlainKeys((key, event) => {
    if (key.toLowerCase() === SECTION_KEY && !event.repeat) toggle();
    if (key === "Escape") {
      if (isDrawing) stopDrawing();
      else setSelected(false);
    }
    if ((key === "Delete" || key === "Backspace") && isSelected && !event.repeat) remove();
  }, isEnabled);

  return { isDrawing, isSelected, draft, setSelected, onPaperPointerDown, toggle, remove };
}

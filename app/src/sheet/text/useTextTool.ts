// Selecting, dragging, editing and adding text on the sheet.
//
// A drag moves the SVG group under the pointer directly, so it follows the
// pointer without waiting for a render, and only the drop changes the look:
// one undo step per move. The moved group keeps its offset until the core's
// new SVG replaces it, so the text never jumps back in between.

import { useEffect, useRef, useState, type RefObject } from "react";
import { arrowStep, usePlainKeys } from "../../lib/keys";
import type { Look } from "../../lib/types";
import type { CanvasControls, PaperView } from "../Canvas";
import { useLiveValue } from "../liveValue";
import { followPointer } from "../../lib/pointerDrag";
import { boxAt } from "../paper";
import { groupFor, readTexts } from "./sheetTexts";
import { isTextBox, withNewBox, withoutBox, withPosition } from "./textEdits";

// Positions are saved in tenths of a millimetre; finer means nothing on paper.
const TENTHS_PER_MM = 10;

export type TextDrag = { id: string; dx: number; dy: number };

type History = { change: (next: Look, mergeKey?: string) => void; seal: () => void };

const rounded = (value: number) => Math.round(value * TENTHS_PER_MM) / TENTHS_PER_MM;

export function useTextTool(look: Look, history: History, canvas: RefObject<CanvasControls | null>, svg: string | null, isEnabled: boolean) {
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [editingId, setEditingId] = useState<string | null>(null);
  const drag = useLiveValue<TextDrag | null>(null);
  const latestLook = useRef(look);
  latestLook.current = look;

  // The core's new SVG has the text where the drop put it.
  useEffect(() => drag.set(null), [drag, svg]);

  // A text box that undo took away is no longer selected.
  const isPresent = selectedId === null || !isTextBox(selectedId) || look.sheet.texts.some((text) => text.id === selectedId);
  useEffect(() => {
    if (!isPresent) {
      setSelectedId(null);
      setEditingId(null);
    }
  }, [isPresent]);

  const onPaperPointerDown = (event: React.PointerEvent, view: PaperView): boolean => {
    // A drop whose render never came (the same picture, or a failed render) must not keep the toolbar hidden.
    drag.set(null);
    const paper = view.getPaper();
    const point = view.toMm(event.clientX, event.clientY);
    const hit = boxAt(readTexts(paper), point.x, point.y);
    if (!hit) {
      setSelectedId(null);
      setEditingId(null);
      return false;
    }

    // Selection shows on press, before any drag.
    event.preventDefault();
    setSelectedId(hit.id);
    setEditingId(null);
    const group = groupFor(paper, hit.id);
    const start = { x: event.clientX, y: event.clientY };
    let offset = { dx: 0, dy: 0 };
    followPointer(event, {
      onDrag: (move) => {
        offset = { dx: (move.clientX - start.x) / view.pxPerMm, dy: (move.clientY - start.y) / view.pxPerMm };
        group?.setAttribute("transform", `translate(${offset.dx} ${offset.dy})`);
        drag.set({ id: hit.id, ...offset });
      },
      onDrop: (_up, isDragged) => {
        if (!isDragged) return;
        const position = { x_mm: rounded(hit.x + offset.dx), y_mm: rounded(hit.y + offset.dy) };
        history.change(withPosition(latestLook.current, hit.id, position));
      },
    });
    return true;
  };

  const onPaperDoubleClick = (event: React.MouseEvent, view: PaperView): boolean => {
    const point = view.toMm(event.clientX, event.clientY);
    const hit = boxAt(readTexts(view.getPaper()), point.x, point.y);
    if (!hit) return false;
    setSelectedId(hit.id);
    setEditingId(hit.id);
    return true;
  };

  /** A new text box in the middle of the view, selected and ready to type into. */
  const addText = () => {
    const centre = canvas.current?.centreMm();
    if (!centre) return;
    const { look: next, id } = withNewBox(latestLook.current, { x_mm: rounded(centre.x), y_mm: rounded(centre.y) });
    // Adding and typing the first words merge into one undo step.
    history.change(next, `text:${id}`);
    setSelectedId(id);
    setEditingId(id);
  };

  const nudge = (id: string, dx: number, dy: number) => {
    // Keys have no pointer to find the paper by; there is only one sheet on screen.
    const text = readTexts(document.querySelector(".paper")).find((entry) => entry.id === id);
    if (!text) return;
    history.change(withPosition(latestLook.current, id, { x_mm: rounded(text.x + dx), y_mm: rounded(text.y + dy) }));
  };

  usePlainKeys((key, event) => {
    if (key.toLowerCase() === "t" && !event.repeat) {
      event.preventDefault();
      addText();
      return;
    }
    if (!selectedId) return;
    if (key === "Escape") setSelectedId(null);
    if (key === "Enter") {
      event.preventDefault();
      setEditingId(selectedId);
    }
    if ((key === "Delete" || key === "Backspace") && isTextBox(selectedId) && !event.repeat) {
      history.change(withoutBox(latestLook.current, selectedId));
      setSelectedId(null);
    }
    const move = arrowStep(event);
    if (move) {
      event.preventDefault();
      nudge(selectedId, move[0], move[1]);
    }
  }, isEnabled && editingId === null);

  return {
    selectedId,
    editingId,
    drag,
    setEditingId,
    setSelectedId,
    onPaperPointerDown,
    onPaperDoubleClick,
    addText,
  };
}

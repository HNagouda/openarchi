// Dragging a plate of the explode view up or down the stack, on the sheet itself.
//
// Like a text drag, the plate's SVG group follows the pointer directly and
// only the drop changes the look: one undo step per move. While it is held,
// the plate is lifted to the front of the stack and a line marks the slot it
// will take. The moved group keeps its place until the core's new SVG
// replaces it, so the plate never jumps back in between.

import { useEffect, useRef, useState } from "react";
import type { Look } from "../../lib/types";
import type { PaperView } from "../Canvas";
import { withPlateMoved } from "../inspector/lookEdits";
import { dropIndex, plateGroupAt, readPlates, type SheetPlate } from "./sheetPlates";

// A press that moves less than this is a click, not a drag.
const DRAG_THRESHOLD_PX = 3;
const LIFTED_CLASS = "is-lifted";

export type PlateDrag = { plates: SheetPlate[]; from: number; to: number };

type History = { change: (next: Look, mergeKey?: string) => void };

export function usePlateDrag(look: Look, history: History, svg: string | null) {
  const [drag, setDrag] = useState<PlateDrag | null>(null);
  const latestLook = useRef(look);
  latestLook.current = look;

  // The core's new SVG has the plates in their new order.
  useEffect(() => setDrag(null), [svg]);

  const onPaperPointerDown = (event: React.PointerEvent, view: PaperView): boolean => {
    if (latestLook.current.view.mode !== "explode") return false;
    const group = plateGroupAt(event.clientX, event.clientY);
    const plates = readPlates(view.getPaper());
    if (!group || plates.length < 2) return false;

    event.preventDefault();
    const from = Number(group.dataset.index);
    const parent = group.parentNode;
    const nextSibling = group.nextSibling;
    const start = { x: event.clientX, y: event.clientY };
    let current: PlateDrag | null = null;

    const lift = () => {
      // Drawn last, so it shows above the plates it passes.
      parent?.appendChild(group);
      group.classList.add(LIFTED_CLASS);
    };
    const putBack = () => {
      group.removeAttribute("transform");
      group.classList.remove(LIFTED_CLASS);
      parent?.insertBefore(group, nextSibling);
    };

    const onMove = (move: PointerEvent) => {
      const distance = Math.hypot(move.clientX - start.x, move.clientY - start.y);
      if (!current && distance < DRAG_THRESHOLD_PX) return;
      if (!current) lift();
      const dy = (move.clientY - start.y) / view.pxPerMm;
      group.setAttribute("transform", `translate(0 ${dy})`);
      current = { plates, from, to: dropIndex(plates, from, plates[from].centreY + dy) };
      setDrag(current);
    };
    const finish = (isDrop: boolean) => {
      window.removeEventListener("pointermove", onMove);
      window.removeEventListener("pointerup", onUp);
      window.removeEventListener("keydown", onKey);
      if (!current) return;
      const isMove = isDrop && current.to !== current.from;
      if (!isMove) {
        putBack();
        setDrag(null);
        return;
      }
      // Leave the plate where it was dropped until the new render arrives.
      group.setAttribute("transform", `translate(0 ${plates[current.to].centreY - plates[from].centreY})`);
      setDrag(null);
      history.change(withPlateMoved(latestLook.current, current.from, current.to));
    };
    const onUp = () => finish(true);
    const onKey = (key: KeyboardEvent) => {
      if (key.key === "Escape") finish(false);
    };
    window.addEventListener("pointermove", onMove);
    window.addEventListener("pointerup", onUp);
    window.addEventListener("keydown", onKey);
    return true;
  };

  return { drag, onPaperPointerDown };
}

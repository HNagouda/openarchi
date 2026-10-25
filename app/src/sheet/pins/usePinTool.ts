// Placing, selecting, dragging, relabelling and deleting pins on the sheet.
//
// Like text, a drag moves the pin's SVG group directly and only the drop
// changes the pins, so a move is one undo step. Positions go through the
// sheet's ground points into RD New metres, so a pin moved on a tilted view
// lands on the same spot of the ground in every other view.

import { useEffect, useRef, useState, type RefObject } from "react";
import { arrowStep, usePlainKeys } from "../../lib/keys";
import type { Pin, PinIcon } from "../../lib/types";
import type { CanvasControls, PaperView } from "../Canvas";
import { useLiveValue } from "../liveValue";
import { followPointer } from "../../lib/pointerDrag";
import { nextPinId, steppedSize, withChangedPin, withoutPin, withPin } from "./pinEdits";
import { boxAt, isOutsideSite, paperPointAt, paperToRd, type Point } from "../paper";
import { pinGroup, readPins } from "./sheetPins";

// A selected pin grows with + or =, which share a key, and shrinks with -.
const SIZE_KEYS: Record<string, 1 | -1> = { "+": 1, "=": 1, "-": -1 };

export type PinDrag = { id: string; dx: number; dy: number };

type Options = {
  pins: Pin[];
  onPins: (next: Pin[], mergeKey?: string) => void;
  canvas: RefObject<CanvasControls | null>;
  svg: string | null;
  isEnabled: boolean;
  /** Says why a pin could not go where it was dropped. */
  onRefused: (message: string) => void;
};

export function usePinTool({ pins, onPins, canvas, svg, isEnabled, onRefused }: Options) {
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [editingId, setEditingId] = useState<string | null>(null);
  const drag = useLiveValue<PinDrag | null>(null);
  const latestPins = useRef(pins);
  latestPins.current = pins;

  // The core's new SVG has the pin where the drop put it.
  useEffect(() => drag.set(null), [drag, svg]);

  // A pin that undo took away is no longer selected.
  const isPresent = selectedId === null || pins.some((pin) => pin.id === selectedId);
  useEffect(() => {
    if (!isPresent) {
      setSelectedId(null);
      setEditingId(null);
    }
  }, [isPresent]);

  /** RD New metres for a paper point, or null with the reason told when no pin can go there. */
  const groundAt = (paper: Element | null, point: Point): Point | null => {
    const toRd = paperToRd(paper);
    if (!toRd) {
      onRefused("This sheet can't take pins until it has been drawn.");
      return null;
    }
    if (isOutsideSite(paper, point)) {
      onRefused("Pins go inside the outline of the sheet's area.");
      return null;
    }
    return toRd(point);
  };

  const placeAt = (icon: PinIcon, paper: Element | null, point: Point) => {
    const place = groundAt(paper, point);
    if (!place) return;
    const id = nextPinId(latestPins.current);
    // Placing and typing the first label merge into one undo step.
    onPins(withPin(latestPins.current, { id, icon, label: "", ...place }), `pin:${id}`);
    setSelectedId(id);
    setEditingId(id);
  };

  /** A pin dragged from the palette and let go at a screen point. */
  const dropAt = (icon: PinIcon, clientX: number, clientY: number) => {
    const hit = paperPointAt(clientX, clientY);
    if (hit) placeAt(icon, hit.paper, hit.point);
  };

  /** A pin chosen from the palette without a drag: in the middle of the view. */
  const placeInView = (icon: PinIcon) => {
    const centre = canvas.current?.centreMm();
    if (centre) placeAt(icon, document.querySelector(".paper"), centre);
  };

  /** Moves the pin, or puts its dragged group back when the spot is refused. */
  const moveTo = (id: string, paper: Element | null, point: Point) => {
    const place = groundAt(paper, point);
    if (place) onPins(withChangedPin(latestPins.current, id, place));
    else pinGroup(paper, id)?.removeAttribute("transform");
  };

  const onPaperPointerDown = (event: React.PointerEvent, view: PaperView): boolean => {
    drag.set(null);
    const paper = view.getPaper();
    const point = view.toMm(event.clientX, event.clientY);
    const hit = boxAt(readPins(paper), point.x, point.y);
    if (!hit) {
      setSelectedId(null);
      setEditingId(null);
      return false;
    }

    event.preventDefault();
    setSelectedId(hit.id);
    setEditingId(null);
    const group = pinGroup(paper, hit.id);
    const start = { x: event.clientX, y: event.clientY };
    let offset = { dx: 0, dy: 0 };
    followPointer(event, {
      onDrag: (move) => {
        offset = { dx: (move.clientX - start.x) / view.pxPerMm, dy: (move.clientY - start.y) / view.pxPerMm };
        group?.setAttribute("transform", `translate(${offset.dx} ${offset.dy})`);
        drag.set({ id: hit.id, ...offset });
      },
      onDrop: (_up, isDragged) => {
        // The moved group keeps its offset until the core's new SVG replaces it.
        if (isDragged) moveTo(hit.id, paper, { x: hit.x + offset.dx, y: hit.y + offset.dy });
      },
    });
    return true;
  };

  const onPaperDoubleClick = (event: React.MouseEvent, view: PaperView): boolean => {
    const point = view.toMm(event.clientX, event.clientY);
    const hit = boxAt(readPins(view.getPaper()), point.x, point.y);
    if (!hit) return false;
    setSelectedId(hit.id);
    setEditingId(hit.id);
    return true;
  };

  const remove = (id: string) => {
    onPins(withoutPin(latestPins.current, id));
    setSelectedId(null);
    setEditingId(null);
  };

  const nudge = (id: string, dx: number, dy: number) => {
    // Keys have no pointer to find the paper by; there is only one sheet on screen.
    const paper = document.querySelector(".paper");
    const pin = readPins(paper).find((entry) => entry.id === id);
    if (pin) moveTo(id, paper, { x: pin.x + dx, y: pin.y + dy });
  };

  usePlainKeys((key, event) => {
    if (!selectedId) return;
    if (key === "Escape") setSelectedId(null);
    if (key === "Enter") {
      event.preventDefault();
      setEditingId(selectedId);
    }
    if ((key === "Delete" || key === "Backspace") && !event.repeat) remove(selectedId);
    const sizeStep = SIZE_KEYS[key];
    const pin = latestPins.current.find((entry) => entry.id === selectedId);
    if (sizeStep && pin) {
      event.preventDefault();
      onPins(withChangedPin(latestPins.current, selectedId, { size: steppedSize(pin.size, sizeStep) }));
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
    setSelectedId,
    setEditingId,
    onPaperPointerDown,
    onPaperDoubleClick,
    dropAt,
    placeInView,
    remove,
  };
}

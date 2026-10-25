// Placing and dragging the View diagram's eye.
//
// With the tool on, one click on the sheet stands the eye there. Without it,
// the eye the core drew can be picked up and dragged. Like pins, only the
// click or the drop changes the look, so every move is one undo step, and
// paper points go through the sheet's ground points into RD New metres, so an
// eye placed on a tilted view stands on the same ground in the plan.

import { useEffect, useRef, useState } from "react";
import { usePlainKeys } from "../../lib/keys";
import type { Look } from "../../lib/types";
import type { PaperView } from "../Canvas";
import { useLiveValue } from "../liveValue";
import { followPointer } from "../pointerDrag";
import { paperToRd, type Point } from "../pins/sheetPins";
import { readEye, withViewPoint } from "./viewPointEdits";

// How close to the eye, in paper millimetres, a press still picks it up.
const EYE_HIT_MM = 3;
export const VIEW_POINT_KEY = "v";

type Options = {
  look: Look;
  onLook: (next: Look) => void;
  svg: string | null;
  /** The sheet can be edited and its diagram draws an eye. */
  isEnabled: boolean;
  /** Says why the eye could not stand where it was put. */
  onRefused: (message: string) => void;
};

export function useViewPointTool({ look, onLook, svg, isEnabled, onRefused }: Options) {
  const [isPlacing, setPlacing] = useState(false);
  // The eye on paper while it is dragged, until the core's new SVG has it.
  const draft = useLiveValue<Point | null>(null);
  const latestLook = useRef(look);
  latestLook.current = look;

  useEffect(() => draft.set(null), [draft, svg]);
  useEffect(() => {
    if (!isEnabled) setPlacing(false);
  }, [isEnabled]);

  /** Stands the eye on the ground under a paper point, or says why it cannot. */
  const standAt = (paper: Element | null, point: Point): boolean => {
    const toRd = paperToRd(paper);
    if (!toRd) {
      onRefused("The viewpoint is placed on a plan or a tilted view.");
      return false;
    }
    onLook(withViewPoint(latestLook.current, toRd(point)));
    return true;
  };

  const drag = (event: React.PointerEvent, view: PaperView) => {
    const paper = view.getPaper();
    followPointer(event, {
      onDrag: (move) => draft.set(view.toMm(move.clientX, move.clientY)),
      onDrop: (up, isDragged) => {
        if (!isDragged) return;
        const isSaved = standAt(paper, view.toMm(up.clientX, up.clientY));
        if (!isSaved) draft.set(null);
      },
    });
  };

  const onPaperPointerDown = (event: React.PointerEvent, view: PaperView): boolean => {
    if (!isEnabled) return false;
    const point = view.toMm(event.clientX, event.clientY);
    if (isPlacing) {
      event.preventDefault();
      standAt(view.getPaper(), point);
      setPlacing(false);
      return true;
    }
    const eye = readEye(view.getPaper());
    if (!eye || Math.hypot(point.x - eye.x, point.y - eye.y) > EYE_HIT_MM) return false;
    event.preventDefault();
    drag(event, view);
    return true;
  };

  const toggle = () => {
    if (isEnabled) setPlacing(!isPlacing);
  };

  usePlainKeys((key, event) => {
    if (key.toLowerCase() === VIEW_POINT_KEY && !event.repeat) toggle();
    if (key === "Escape") setPlacing(false);
  }, isEnabled);

  return { isPlacing, draft, toggle, onPaperPointerDown };
}

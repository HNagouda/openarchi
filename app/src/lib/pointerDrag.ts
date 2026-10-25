// A press that may turn into a drag. The pointer has to travel a few pixels
// first, so a click that wobbles stays a click. Everything that drags in the
// app uses the same distance, so pins, text, tabs, rows and blocks all feel the same.

// A press that moves less than this is a click, not a drag.
export const DRAG_THRESHOLD_PX = 4;

type Handlers = {
  /** Every move once the pointer has travelled past the threshold. */
  onDrag: (move: PointerEvent) => void;
  /** The release, and whether the press became a drag. */
  onDrop: (up: PointerEvent, isDragged: boolean) => void;
};

/** Follows the pointer from a press until it lets go. */
export function followPointer(press: { clientX: number; clientY: number }, { onDrag, onDrop }: Handlers) {
  const start = { x: press.clientX, y: press.clientY };
  let isDragging = false;

  const onMove = (move: PointerEvent) => {
    if (!isDragging && Math.hypot(move.clientX - start.x, move.clientY - start.y) < DRAG_THRESHOLD_PX) return;
    isDragging = true;
    onDrag(move);
  };
  const onUp = (up: PointerEvent) => {
    window.removeEventListener("pointermove", onMove);
    window.removeEventListener("pointerup", onUp);
    onDrop(up, isDragging);
  };

  window.addEventListener("pointermove", onMove);
  window.addEventListener("pointerup", onUp);
}

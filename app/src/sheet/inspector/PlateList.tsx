// The explode view's plates, top first, reordered by dragging a row by its
// grip. The row lifts and follows the pointer, the rows it passes slide
// aside to open the gap it will land in, and only the drop changes the look:
// one undo step per move. With a row focused, Alt+Up and Alt+Down move it
// one place.

import { useEffect, useRef, useState } from "react";
import type { Diagram, Look } from "../../lib/types";
import { plateName } from "../../lib/diagrams";
import { Button } from "../../ui/Button";
import { Icon } from "../../ui/Icon";
import { withPlateMoved, withView } from "./lookEdits";
import "./plates.css";

// A press that moves less than this is a click, not a drag.
const DRAG_THRESHOLD_PX = 3;

type Drag = { from: number; to: number; dy: number; rowHeight: number };

type Props = {
  diagrams: Diagram[];
  look: Look;
  onLook: (next: Look) => void;
};

export function PlateList({ diagrams, look, onLook }: Props) {
  const plates = look.view.plates;
  const list = useRef<HTMLOListElement>(null);
  const [drag, setDrag] = useState<Drag | null>(null);
  // The plate a key just moved, so focus follows it to its new row.
  const movedByKey = useRef<string | null>(null);

  useEffect(() => {
    const id = movedByKey.current;
    movedByKey.current = null;
    if (id) list.current?.querySelector<HTMLElement>(`[data-plate-row="${CSS.escape(id)}"]`)?.focus();
  }, [plates]);

  const startDrag = (event: React.PointerEvent, from: number) => {
    if (event.button !== 0 || plates.length < 2) return;
    event.preventDefault();
    const rows = [...(list.current?.querySelectorAll<HTMLElement>("[data-plate-row]") ?? [])];
    const centres = rows.map((row) => {
      const rect = row.getBoundingClientRect();
      return rect.top + rect.height / 2;
    });
    const rowHeight = rows[from]?.getBoundingClientRect().height ?? 0;
    const startY = event.clientY;
    let current: Drag | null = null;

    const onMove = (move: PointerEvent) => {
      const dy = move.clientY - startY;
      if (!current && Math.abs(dy) < DRAG_THRESHOLD_PX) return;
      current = { from, to: dropIndex(centres, from, centres[from] + dy), dy, rowHeight };
      setDrag(current);
    };
    const finish = (isDrop: boolean) => {
      window.removeEventListener("pointermove", onMove);
      window.removeEventListener("pointerup", onUp);
      window.removeEventListener("keydown", onKey);
      setDrag(null);
      if (isDrop && current && current.to !== current.from) onLook(withPlateMoved(look, current.from, current.to));
    };
    const onUp = () => finish(true);
    const onKey = (key: KeyboardEvent) => {
      if (key.key === "Escape") finish(false);
    };
    window.addEventListener("pointermove", onMove);
    window.addEventListener("pointerup", onUp);
    window.addEventListener("keydown", onKey);
  };

  const moveByKey = (event: React.KeyboardEvent, index: number) => {
    if (!event.altKey || (event.key !== "ArrowUp" && event.key !== "ArrowDown")) return;
    event.preventDefault();
    const to = event.key === "ArrowUp" ? index - 1 : index + 1;
    if (to < 0 || to >= plates.length) return;
    movedByKey.current = plates[index];
    onLook(withPlateMoved(look, index, to));
  };

  return (
    <ol className={drag ? "plate-list is-dragging" : "plate-list"} ref={list} aria-label="Plates, top first">
      {plates.map((id, index) => {
        const isLifted = drag?.from === index;
        return (
          <li
            key={id}
            data-plate-row={id}
            className={isLifted ? "plate-row is-lifted" : "plate-row"}
            style={drag ? { transform: `translateY(${rowOffset(drag, index)}px)` } : undefined}
            tabIndex={0}
            aria-label={`${plateName(diagrams, id)}, plate ${index + 1} of ${plates.length}. Alt and the arrow keys move it.`}
            onKeyDown={(event) => moveByKey(event, index)}
          >
            <span className="plate-grip" data-tip="Drag to reorder" onPointerDown={(event) => startDrag(event, index)}>
              <Icon name="grip" />
            </span>
            <span className="plate-name">{plateName(diagrams, id)}</span>
            <Button
              icon="close"
              tip="Remove plate"
              disabled={plates.length === 1}
              onClick={() => onLook(withView(look, { plates: plates.filter((plate) => plate !== id) }))}
            />
          </li>
        );
      })}
    </ol>
  );
}

/** How far a row sits from its place during a drag: the held row follows the pointer, the rows it passed make room. */
function rowOffset(drag: Drag, index: number): number {
  if (index === drag.from) return drag.dy;
  if (index > drag.from && index <= drag.to) return -drag.rowHeight;
  if (index < drag.from && index >= drag.to) return drag.rowHeight;
  return 0;
}

/** Where a row dragged from `from` lands when its middle is at `y`: past every other row whose middle it has crossed. */
function dropIndex(centres: number[], from: number, y: number): number {
  return centres.filter((centre, index) => index !== from && centre < y).length;
}

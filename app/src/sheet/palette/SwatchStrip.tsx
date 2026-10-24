// The palette's swatches in a row. Click one to edit it; drag one along the
// row to reorder. From the keyboard: arrows choose, Alt with an arrow moves
// the chosen swatch, Delete removes it.

import { useRef, useState, type KeyboardEvent, type PointerEvent } from "react";
import type { PaletteSwatch } from "../../lib/palette";
import { Icon } from "../../ui/Icon";
import { MAX_SWATCHES, MIN_SWATCHES } from "./draftEdits";

// A press that moves less than this is a click, not a drag.
const DRAG_THRESHOLD_PX = 4;

type Props = {
  swatches: PaletteSwatch[];
  selected: number;
  onSelect: (index: number) => void;
  onMove: (from: number, to: number) => void;
  onRemove: (index: number) => void;
  onAdd: () => void;
  /** The roles each swatch fills, as short labels under it. */
  rolesOf: (index: number) => string[];
};

type Drag = { from: number; startX: number; over: number; isDragging: boolean };

export function SwatchStrip({ swatches, selected, onSelect, onMove, onRemove, onAdd, rolesOf }: Props) {
  const row = useRef<HTMLDivElement>(null);
  const [drag, setDrag] = useState<Drag | null>(null);

  /** The slot under the pointer, from the swatches' own positions. */
  const slotAt = (clientX: number) => {
    const cells = [...(row.current?.querySelectorAll<HTMLElement>(".swatch-cell") ?? [])];
    const index = cells.findIndex((cell) => clientX < cell.getBoundingClientRect().left + cell.offsetWidth / 2);
    return index === -1 ? cells.length - 1 : index;
  };

  const onPointerDown = (event: PointerEvent<HTMLButtonElement>, index: number) => {
    if (event.button !== 0) return;
    event.currentTarget.setPointerCapture(event.pointerId);
    setDrag({ from: index, startX: event.clientX, over: index, isDragging: false });
  };

  const onPointerMove = (event: PointerEvent<HTMLButtonElement>) => {
    if (!drag) return;
    const isDragging = drag.isDragging || Math.abs(event.clientX - drag.startX) > DRAG_THRESHOLD_PX;
    setDrag({ ...drag, isDragging, over: isDragging ? slotAt(event.clientX) : drag.over });
  };

  const onPointerUp = () => {
    if (!drag) return;
    if (drag.isDragging) onMove(drag.from, drag.over);
    else onSelect(drag.from);
    setDrag(null);
  };

  const onKey = (event: KeyboardEvent, index: number) => {
    const step = event.key === "ArrowLeft" ? -1 : event.key === "ArrowRight" ? 1 : 0;
    if (step !== 0 && event.altKey) onMove(index, index + step);
    else if (step !== 0) onSelect(Math.min(swatches.length - 1, Math.max(0, index + step)));
    else if (event.key === "Delete" || event.key === "Backspace") onRemove(index);
    else return;
    event.preventDefault();
  };

  return (
    <div className="swatch-strip" ref={row} role="listbox" aria-label="Swatches" aria-orientation="horizontal">
      {swatches.map((swatch, index) => {
        const isMoving = drag?.isDragging && drag.from === index;
        const isTarget = drag?.isDragging && drag.over === index && drag.from !== index;
        const classes = ["swatch-cell", index === selected ? "is-selected" : "", isMoving ? "is-moving" : "", isTarget ? "is-target" : ""];
        const roles = rolesOf(index);
        return (
          <div key={index} className={classes.filter(Boolean).join(" ")}>
            <button
              type="button"
              role="option"
              aria-selected={index === selected}
              className="swatch-chip"
              style={{ background: swatch.hex }}
              data-tip={swatch.name ? `${swatch.name}, ${swatch.hex}` : swatch.hex}
              onPointerDown={(event) => onPointerDown(event, index)}
              onPointerMove={onPointerMove}
              onPointerUp={onPointerUp}
              onPointerCancel={() => setDrag(null)}
              onKeyDown={(event) => onKey(event, index)}
            />
            <span className="swatch-hex tabular">{swatch.hex.replace("#", "")}</span>
            <span className="swatch-roles">{roles.join(", ")}</span>
            {swatches.length > MIN_SWATCHES && (
              <button type="button" className="swatch-remove" aria-label={`Remove ${swatch.hex}`} data-tip="Remove" onClick={() => onRemove(index)}>
                <Icon name="close" size={10} />
              </button>
            )}
          </div>
        );
      })}
      {swatches.length < MAX_SWATCHES && (
        <button type="button" className="swatch-add" aria-label="Add a swatch" data-tip="Add a swatch" onClick={onAdd}>
          <Icon name="plus" />
        </button>
      )}
    </div>
  );
}

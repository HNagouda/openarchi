// The markers a pin can be, in the tab bar. Drag one onto the sheet to place
// it there; a click or Return places it in the middle of the view.

import { useState } from "react";
import type { PinIcon } from "../../lib/types";
import { followPointer } from "../../lib/pointerDrag";
import { PinGlyph } from "./PinGlyph";
import { PIN_ICONS } from "./pinEdits";
import "./pins.css";

type Ghost = { icon: PinIcon; x: number; y: number };

type Props = {
  isDisabled: boolean;
  /** Why the palette is off, for its tooltip. */
  disabledTip: string;
  onDrop: (icon: PinIcon, clientX: number, clientY: number) => void;
  onPlace: (icon: PinIcon) => void;
};

export function PinPalette({ isDisabled, disabledTip, onDrop, onPlace }: Props) {
  const [ghost, setGhost] = useState<Ghost | null>(null);

  const startDrag = (event: React.PointerEvent, icon: PinIcon) => {
    if (event.button !== 0 || isDisabled) return;
    event.preventDefault();
    followPointer(event, {
      onDrag: (move) => setGhost({ icon, x: move.clientX, y: move.clientY }),
      onDrop: (up, isDragged) => {
        setGhost(null);
        if (isDragged) onDrop(icon, up.clientX, up.clientY);
        else onPlace(icon);
      },
    });
  };

  return (
    <div className="pin-palette" role="toolbar" aria-label="Pins">
      {PIN_ICONS.map(({ icon, label }) => (
        <button
          key={icon}
          type="button"
          className="pin-palette-item"
          disabled={isDisabled}
          data-tip={isDisabled ? disabledTip : `${label}: drag onto the sheet`}
          data-tip-side="above"
          aria-label={label}
          onPointerDown={(event) => startDrag(event, icon)}
          onKeyDown={(event) => {
            if (event.key !== "Enter" && event.key !== " ") return;
            event.preventDefault();
            onPlace(icon);
          }}
        >
          <PinGlyph icon={icon} />
        </button>
      ))}
      {ghost && (
        // A pin and a flag stand on the pointer; the other markers sit centred on it.
        <div className={ghost.icon === "pin" || ghost.icon === "flag" ? "pin-ghost is-standing" : "pin-ghost"} style={{ left: ghost.x, top: ghost.y }}>
          <PinGlyph icon={ghost.icon} size={24} />
        </div>
      )}
    </div>
  );
}

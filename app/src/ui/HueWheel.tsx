// A hue ring: drag round it or use the arrow keys. Red sits at the top and
// the hues run clockwise, as CSS draws a conic gradient. Whatever sits inside
// the ring (the picker's square) is passed as children.

import type { KeyboardEvent, PointerEvent, ReactNode } from "react";

const FULL_TURN = 360;
// Arrow keys turn by 1°, with Shift by 15°.
const KEY_STEP = 1;
const KEY_STEP_LARGE = 15;
// The ring's thickness, as colorPicker.css draws it; presses inside it belong to the centre.
const RING_PX = 16;

type Props = { hue: number; onHue: (hue: number) => void; children?: ReactNode };

export function HueWheel({ hue, onHue, children }: Props) {
  const pick = (event: PointerEvent<HTMLDivElement>) => {
    const rect = event.currentTarget.getBoundingClientRect();
    const dx = event.clientX - (rect.left + rect.width / 2);
    const dy = event.clientY - (rect.top + rect.height / 2);
    const degrees = (Math.atan2(dx, -dy) * 180) / Math.PI;
    onHue((degrees + FULL_TURN) % FULL_TURN);
  };

  const onKey = (event: KeyboardEvent) => {
    const step = event.shiftKey ? KEY_STEP_LARGE : KEY_STEP;
    let turn = 0;
    if (event.key === "ArrowRight" || event.key === "ArrowUp") turn = step;
    else if (event.key === "ArrowLeft" || event.key === "ArrowDown") turn = -step;
    else return;
    event.preventDefault();
    onHue((hue + turn + FULL_TURN) % FULL_TURN);
  };

  return (
    <div className="hue-wheel">
      <div
        className="hue-ring"
        tabIndex={0}
        role="slider"
        aria-label="Hue"
        aria-valuemin={0}
        aria-valuemax={FULL_TURN}
        aria-valuenow={Math.round(hue)}
        onPointerDown={(event) => {
          if (!isOnRing(event)) return;
          event.currentTarget.setPointerCapture(event.pointerId);
          event.currentTarget.focus();
          pick(event);
        }}
        onPointerMove={(event) => {
          if (event.currentTarget.hasPointerCapture(event.pointerId)) pick(event);
        }}
        onKeyDown={onKey}
      >
        <span className="hue-ring-thumb" style={{ transform: `rotate(${hue}deg)` }} />
      </div>
      <div className="hue-wheel-centre">{children}</div>
    </div>
  );
}

function isOnRing(event: PointerEvent<HTMLDivElement>) {
  const rect = event.currentTarget.getBoundingClientRect();
  const radius = rect.width / 2;
  const distance = Math.hypot(event.clientX - (rect.left + radius), event.clientY - (rect.top + radius));
  return distance >= radius - RING_PX && distance <= radius;
}

// The View diagram's viewpoint in the inspector: the site centre, one of the
// project's pins, or a point placed on the sheet. Placing and dragging happen
// on the sheet; the core works out what the eye sees.

import { shortcutLabel } from "../../lib/platform";
import type { Look, Pin } from "../../lib/types";
import { Button } from "../../ui/Button";
import { Section } from "../../ui/Section";
import { VIEW_POINT_KEY } from "./useViewPointTool";
import { pinUnderEye, withViewPin, withViewPoint } from "./viewPointEdits";
import "./viewpoint.css";

// The menu's values besides the pins, since an <option> value is a string.
const CENTRE = "centre";
const PLACED = "placed";
const PIN_PREFIX = "pin:";

/** What the sheet step hands the inspector for the viewpoint. */
export type ViewPointControls = {
  pins: Pin[];
  isPlacing: boolean;
  onPlace: () => void;
};

type Props = {
  look: Look;
  controls: ViewPointControls;
  onLook: (next: Look) => void;
};

export function ViewPointSection({ look, controls, onLook }: Props) {
  const { pins, isPlacing, onPlace } = controls;
  const pin = pinUnderEye(look, pins);
  const value = pin ? `${PIN_PREFIX}${pin.id}` : look.view_point && !look.view_pin ? PLACED : CENTRE;

  const choose = (chosen: string) => {
    if (chosen === CENTRE) onLook(withViewPoint(look, null));
    const picked = pins.find((entry) => `${PIN_PREFIX}${entry.id}` === chosen);
    if (picked) onLook(withViewPin(look, picked.id));
  };

  return (
    <Section id="view-point" title="Viewpoint">
      <div className="sheet-fields">
        <label className="field-stack">
          <span className="field-label">The eye stands at</span>
          <select className="text-input" value={value} onChange={(event) => choose(event.target.value)}>
            <option value={CENTRE}>Site centre</option>
            {pins.map((entry) => (
              <option key={entry.id} value={`${PIN_PREFIX}${entry.id}`}>
                Pin: {entry.label || entry.id}
              </option>
            ))}
            {value === PLACED && (
              <option value={PLACED} disabled>
                A point placed on the sheet
              </option>
            )}
          </select>
        </label>
        <Button icon="eye" tip="Click the sheet where the eye should stand" shortcut={VIEW_POINT_KEY} isActive={isPlacing} onClick={onPlace}>
          {isPlacing ? "Click the sheet…" : "Place on the sheet"}
        </Button>
        <p className="view-point-hint">
          Or drag the eye on the sheet ({shortcutLabel(VIEW_POINT_KEY).toUpperCase()} places it). The site centre stands it on the street or square nearest the middle; on a pin, the eye follows the pin.
        </p>
      </div>
    </Section>
  );
}

import type { Diagram, Look, ViewMode } from "../../lib/types";
import { Button } from "../../ui/Button";
import { NumberField } from "../../ui/NumberField";
import { Section } from "../../ui/Section";
import { Segmented } from "../../ui/Segmented";
import { Toggle } from "../../ui/Toggle";
import { withPlateMoved, withSun, withView } from "./lookEdits";

const MODES: { value: ViewMode; label: string; tip: string }[] = [
  { value: "plan", label: "Plan", tip: "Straight down" },
  { value: "axonometric", label: "Axo", tip: "Tilted, with buildings at their height" },
  { value: "explode", label: "Explode", tip: "Several diagrams stacked as plates" },
];

const MAX_ROTATION = 355;
const ROTATION_STEP = 5;
/** Sun dates are taken in one year by the core, so the date field only needs a placeholder year. */
const PLACEHOLDER_YEAR = 2026;

type Props = {
  diagrams: Diagram[];
  look: Look;
  onLook: (next: Look, mergeKey?: string) => void;
  onSeal: () => void;
};

const pad = (value: number) => String(value).padStart(2, "0");

/** Plan, axonometric or explode, plus the sun's date and time where it shows. */
export function ViewSection({ diagrams, look, onLook, onSeal }: Props) {
  const view = look.view;
  const diagram = diagrams.find((entry) => entry.id === look.diagram_id);
  const isTilted = view.mode !== "plan";
  const hasSun = view.mode === "axonometric" || Boolean(diagram?.solar);
  const nameOf = (id: string) => diagrams.find((entry) => entry.id === id)?.name ?? id;
  const unused = diagrams.filter((entry) => !view.plates.includes(entry.id));

  return (
    <Section id="view" title="View">
      <div className="sheet-fields">
        <Segmented label="View" value={view.mode} options={MODES} onChange={(mode) => onLook(withView(look, { mode }))} />

        {isTilted && (
          <div className="field-stack">
            <span className="field-label">Rotation</span>
            <NumberField
              label="Rotation"
              value={view.rotation_deg}
              min={0}
              max={MAX_ROTATION}
              step={ROTATION_STEP}
              unit="°"
              onChange={(rotation_deg) => onLook(withView(look, { rotation_deg }), "rotation")}
              onSeal={onSeal}
            />
          </div>
        )}

        {view.mode === "explode" && (
          <div className="field-stack">
            <span className="field-label">Plates, top first</span>
            {view.plates.map((id, index) => (
              <div key={id} className="layer-row">
                <span className="layer-main">{nameOf(id)}</span>
                <Button icon="chevronUp" tip="Move up" disabled={index === 0} onClick={() => onLook(withPlateMoved(look, index, -1))} />
                <Button
                  icon="chevronDown"
                  tip="Move down"
                  disabled={index === view.plates.length - 1}
                  onClick={() => onLook(withPlateMoved(look, index, 1))}
                />
                <Button
                  icon="close"
                  tip="Remove plate"
                  disabled={view.plates.length === 1}
                  onClick={() => onLook(withView(look, { plates: view.plates.filter((plate) => plate !== id) }))}
                />
              </div>
            ))}
            {unused.length > 0 && (
              <select
                className="text-input"
                aria-label="Add a plate"
                value=""
                onChange={(event) => onLook(withView(look, { plates: [...view.plates, event.target.value] }))}
              >
                <option value="" disabled>
                  Add a plate…
                </option>
                {unused.map((entry) => (
                  <option key={entry.id} value={entry.id}>
                    {entry.name}
                  </option>
                ))}
              </select>
            )}
            <Toggle label="Stacked close" isOn={view.stacking} onChange={(stacking) => onLook(withView(look, { stacking }))} />
          </div>
        )}

        {hasSun && (
          <div className="field-pair">
            <div className="field-stack">
              <span className="field-label">Sun date</span>
              <input
                type="date"
                className="text-input"
                value={`${PLACEHOLDER_YEAR}-${pad(look.sun.month)}-${pad(look.sun.day)}`}
                min={`${PLACEHOLDER_YEAR}-01-01`}
                max={`${PLACEHOLDER_YEAR}-12-31`}
                onChange={(event) => {
                  const [, month, day] = event.target.value.split("-").map(Number);
                  if (month && day) onLook(withSun(look, { month, day }));
                }}
              />
            </div>
            <div className="field-stack field-narrow">
              <span className="field-label">Time</span>
              <input
                type="time"
                className="text-input"
                value={`${pad(look.sun.hour)}:${pad(look.sun.minute)}`}
                onChange={(event) => {
                  const [hour, minute] = event.target.value.split(":").map(Number);
                  if (Number.isFinite(hour) && Number.isFinite(minute)) onLook(withSun(look, { hour, minute }));
                }}
              />
            </div>
          </div>
        )}
      </div>
    </Section>
  );
}

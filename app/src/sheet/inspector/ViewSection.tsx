import type { Diagram, Look, ViewMode, WindSeason } from "../../lib/types";
import { NumberField } from "../../ui/NumberField";
import { Section } from "../../ui/Section";
import { Segmented } from "../../ui/Segmented";
import { Toggle } from "../../ui/Toggle";
import { plateOptions } from "../../lib/diagrams";
import { withSheet, withSun, withView } from "./lookEdits";
import { PlateList } from "./PlateList";
import { SectionFields } from "../section/SectionFields";

const MODES: { value: ViewMode; label: string; tip: string }[] = [
  { value: "plan", label: "Plan", tip: "Straight down" },
  { value: "axonometric", label: "Axo", tip: "Tilted, with buildings at their height" },
  { value: "explode", label: "Explode", tip: "Several diagrams stacked as plates" },
  { value: "section", label: "Section", tip: "An elevation along the sheet's section line" },
];
const NO_LINE_TIP = "Draw a section line on the sheet first";
const PLAN_ONLY_TIP = "This diagram is worked out in plan only";
const TILTED_MODES: ViewMode[] = ["axonometric", "explode"];
const SEASONS: { value: WindSeason; label: string; tip: string }[] = [
  { value: "year", label: "Year", tip: "The most common wind of the whole year" },
  { value: "winter", label: "Winter", tip: "December to February" },
  { value: "spring", label: "Spring", tip: "March to May" },
  { value: "summer", label: "Summer", tip: "June to August" },
  { value: "autumn", label: "Autumn", tip: "September to November" },
];

const MAX_ROTATION = 355;
/** Below this the neighbourhood is all but gone; the toggle hides it outright. */
const MIN_CONTEXT_OPACITY = 0.1;
const CONTEXT_OPACITY_STEP = 0.05;
const PERCENT = 100;
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

/** Plan, axonometric, explode or section, whether the neighbourhood shows and how strongly, the sun's date and time where it shows, the wind's season, and the section line. */
export function ViewSection({ diagrams, look, onLook, onSeal }: Props) {
  const view = look.view;
  const diagram = diagrams.find((entry) => entry.id === look.diagram_id);
  const isTilted = view.mode === "axonometric" || view.mode === "explode";
  // The section view needs a line to cut along; until one is drawn it is greyed out.
  // A plan-only diagram, such as the wind, greys out the tilted views for good.
  const modes = MODES.map((mode) => {
    if (mode.value === "section" && !look.section) return { ...mode, isDisabled: true, tip: NO_LINE_TIP };
    if (diagram?.plan_only && TILTED_MODES.includes(mode.value)) return { ...mode, isDisabled: true, tip: PLAN_ONLY_TIP };
    return mode;
  });
  const hasWind = diagram?.layers.includes("wind") === true;
  const hasSun = view.mode === "axonometric" || Boolean(diagram?.solar);
  // Hours of sun add up a whole day, so only a tilted view, which is lit at one moment, needs the time.
  const hasTime = view.mode === "axonometric" || diagram?.solar !== "sun_hours";
  const unused = plateOptions(diagrams).filter((entry) => !view.plates.includes(entry.id));

  return (
    <Section id="view" title="View">
      <div className="sheet-fields">
        <Segmented label="View" value={view.mode} options={modes} onChange={(mode) => onLook(withView(look, { mode }))} />
        {/* The neighbourhood around the site, and around a sheet's smaller area the rest of the site too. */}
        <Toggle label="Neighbourhood" isOn={look.sheet.show_context} onChange={(show_context) => onLook(withSheet(look, { show_context }))} />
        {look.sheet.show_context && (
          <div className="field-stack">
            <span className="field-label">Neighbourhood opacity</span>
            <NumberField
              label="Neighbourhood opacity"
              value={look.sheet.context_opacity}
              min={MIN_CONTEXT_OPACITY}
              max={1}
              step={CONTEXT_OPACITY_STEP}
              unit="%"
              display={(value) => String(Math.round(value * PERCENT))}
              read={(typed) => typed / PERCENT}
              onChange={(context_opacity) => onLook(withSheet(look, { context_opacity }), "context_opacity")}
              onSeal={onSeal}
            />
          </div>
        )}

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
            <PlateList diagrams={diagrams} look={look} onLook={onLook} />
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
            {hasTime && (
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
            )}
          </div>
        )}

        {hasWind && (
          <div className="field-stack">
            <span className="field-label">Prevailing wind of</span>
            <Segmented label="Prevailing wind of" value={look.wind_season ?? "year"} options={SEASONS} onChange={(wind_season) => onLook({ ...look, wind_season })} />
          </div>
        )}

        <SectionFields look={look} onLook={onLook} onSeal={onSeal} />
      </div>
    </Section>
  );
}

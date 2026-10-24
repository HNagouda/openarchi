// The laser cutting part of the export sheet: the model's scale, the board,
// the laser bed and which parts to cut, with what the files will hold.
// Settings are preferences, kept per machine and outside undo, like the
// other export settings; only the scale starts from the sheet each time.

import { useEffect, useState } from "react";
import { errorText, laserSummary } from "../lib/ipc";
import { formatScale } from "../lib/format";
import { readPreference, writePreference } from "../lib/storage";
import type { LaserFormat, LaserSettings, LaserSummary, Look } from "../lib/types";
import { NumberField } from "../ui/NumberField";
import { Segmented } from "../ui/Segmented";
import { Toggle } from "../ui/Toggle";

// The same scales and defaults as crates/core/src/export/laser/mod.rs, which the assistant uses.
const MODEL_SCALES = [200, 500, 1000, 2000, 2500];
const DEFAULT_SCALE = 1000;
const SCALE_PRESETS = [500, 1000, 2500];
const MIN_SCALE = 100;
const MAX_SCALE = 10_000;
const SCALE_STEP = 50;
const THICKNESS_PRESETS = [
  { mm: 2, label: "2 mm board" },
  { mm: 3, label: "3 mm MDF" },
];
const MIN_THICKNESS_MM = 0.5;
const MAX_THICKNESS_MM = 20;
const THICKNESS_STEP_MM = 0.1;
const BED_PRESETS = [
  [600, 400],
  [900, 600],
  [1000, 600],
];
const MIN_BED_MM = 100;
const MAX_BED_MM = 3000;
const BED_STEP_MM = 10;
// From crates/core/src/export/laser/pieces.rs and render/solid.rs, for the summary's wording.
const MIN_PIECE_MM = 2;
const DEFAULT_HEIGHT_M = 9;
// Long enough that dragging a slider asks the core once, at the end.
const SUMMARY_DELAY_MS = 250;
const OTHER = "other";

type Custom = { scale: boolean; thickness: boolean; bed: boolean };

type Stored = Omit<LaserSettings, "scale">;
const STORED_DEFAULTS: Stored = {
  thickness_mm: 2,
  bed_width_mm: 600,
  bed_height_mm: 400,
  has_base_plate: true,
  has_buildings: true,
  has_contours: true,
  format: "svg",
};

/** The settings, starting from the last ones used and the sheet's scale. */
export function useLaserSettings(look: Look): [LaserSettings, (patch: Partial<LaserSettings>) => void] {
  const [settings, setSettings] = useState<LaserSettings>(() => ({
    ...STORED_DEFAULTS,
    ...readPreference<Partial<Stored>>("laser", {}),
    scale: MODEL_SCALES.includes(look.sheet.scale) ? look.sheet.scale : DEFAULT_SCALE,
  }));
  const update = (patch: Partial<LaserSettings>) => {
    const next = { ...settings, ...patch };
    setSettings(next);
    const { scale: _scale, ...stored } = next;
    writePreference("laser", stored);
  };

  return [settings, update];
}

type Props = { look: Look; settings: LaserSettings; onChange: (patch: Partial<LaserSettings>) => void };

export function LaserFields({ look, settings, onChange }: Props) {
  const [summary, setSummary] = useState<LaserSummary | null>(null);
  const [problem, setProblem] = useState<string | null>(null);
  // Which fields show their number instead of a preset; a custom value that matches a preset stays custom.
  const [custom, setCustom] = useState<Custom>(() => ({
    scale: !SCALE_PRESETS.includes(settings.scale),
    thickness: !THICKNESS_PRESETS.some((preset) => preset.mm === settings.thickness_mm),
    bed: !BED_PRESETS.some(([width, height]) => width === settings.bed_width_mm && height === settings.bed_height_mm),
  }));

  useEffect(() => {
    let isCurrent = true;
    const timer = setTimeout(() => {
      laserSummary(look, settings)
        .then((next) => {
          if (!isCurrent) return;
          setSummary(next);
          setProblem(null);
        })
        .catch((reason) => {
          if (isCurrent) setProblem(errorText(reason));
        });
    }, SUMMARY_DELAY_MS);
    return () => {
      isCurrent = false;
      clearTimeout(timer);
    };
  }, [look, settings]);

  const scalePreset = custom.scale ? OTHER : settings.scale;
  const thicknessPreset = custom.thickness ? OTHER : settings.thickness_mm;
  const bedIndex = BED_PRESETS.findIndex(([width, height]) => width === settings.bed_width_mm && height === settings.bed_height_mm);
  const bedPreset = custom.bed ? OTHER : bedIndex;

  return (
    <>
      <div className="field-stack">
        <span className="field-label">Model scale</span>
        <Segmented
          label="Model scale"
          value={scalePreset}
          options={[...SCALE_PRESETS.map((scale) => ({ value: scale as number | string, label: formatScale(scale) })), { value: OTHER, label: "Other" }]}
          onChange={(value) => {
            setCustom({ ...custom, scale: value === OTHER });
            if (value !== OTHER) onChange({ scale: Number(value) });
          }}
        />
        {scalePreset === OTHER && (
          <NumberField
            label="Model scale"
            value={settings.scale}
            min={MIN_SCALE}
            max={MAX_SCALE}
            step={SCALE_STEP}
            isLogarithmic
            display={formatScale}
            onChange={(scale) => onChange({ scale })}
          />
        )}
      </div>
      <div className="field-stack">
        <span className="field-label">Material</span>
        <Segmented
          label="Material"
          value={thicknessPreset}
          options={[...THICKNESS_PRESETS.map((preset) => ({ value: preset.mm as number | string, label: preset.label })), { value: OTHER, label: "Other" }]}
          onChange={(value) => {
            setCustom({ ...custom, thickness: value === OTHER });
            if (value !== OTHER) onChange({ thickness_mm: Number(value) });
          }}
        />
        {thicknessPreset === OTHER && (
          <NumberField
            label="Thickness"
            value={settings.thickness_mm}
            min={MIN_THICKNESS_MM}
            max={MAX_THICKNESS_MM}
            step={THICKNESS_STEP_MM}
            unit="mm"
            onChange={(thickness_mm) => onChange({ thickness_mm })}
          />
        )}
      </div>
      <div className="field-stack">
        <span className="field-label">Laser bed</span>
        <Segmented
          label="Laser bed"
          value={bedPreset}
          options={[
            ...BED_PRESETS.map(([width, height], index) => ({ value: index as number | string, label: `${width} × ${height}` })),
            { value: OTHER, label: "Custom" },
          ]}
          onChange={(value) => {
            setCustom({ ...custom, bed: value === OTHER });
            if (value !== OTHER) onChange({ bed_width_mm: BED_PRESETS[Number(value)][0], bed_height_mm: BED_PRESETS[Number(value)][1] });
          }}
        />
        {bedPreset === OTHER && (
          <>
            <span className="field-note">Width</span>
            <NumberField label="Bed width" value={settings.bed_width_mm} min={MIN_BED_MM} max={MAX_BED_MM} step={BED_STEP_MM} unit="mm" onChange={(bed_width_mm) => onChange({ bed_width_mm })} />
            <span className="field-note">Depth</span>
            <NumberField label="Bed depth" value={settings.bed_height_mm} min={MIN_BED_MM} max={MAX_BED_MM} step={BED_STEP_MM} unit="mm" onChange={(bed_height_mm) => onChange({ bed_height_mm })} />
          </>
        )}
      </div>
      <div className="field-stack">
        <span className="field-label">Parts</span>
        <div className="toggles">
          <Toggle label="Base plate, with streets and footprints scored" isOn={settings.has_base_plate} onChange={(has_base_plate) => onChange({ has_base_plate })} />
          <Toggle label="Buildings, stacked in layers" isOn={settings.has_buildings} onChange={(has_buildings) => onChange({ has_buildings })} />
          {summary?.has_contours && settings.has_base_plate && (
            <Toggle label="Contour lines on the base plate" isOn={settings.has_contours} onChange={(has_contours) => onChange({ has_contours })} />
          )}
        </div>
      </div>
      <div className="field-stack">
        <span className="field-label">File</span>
        <Segmented<LaserFormat>
          label="File"
          value={settings.format}
          options={[
            { value: "svg", label: "SVG" },
            { value: "dxf", label: "DXF" },
          ]}
          onChange={(format) => onChange({ format })}
        />
        <span className="field-note">One file per laser bed. Cuts are red hairlines, scores blue.</span>
      </div>
      {problem ? <p className="export-error">{problem}</p> : summary && <p className="laser-summary">{summaryText(summary, settings)}</p>}
    </>
  );
}

const area = new Intl.NumberFormat("en-GB", { maximumFractionDigits: 2 });
const count = new Intl.NumberFormat("en-GB");

/** What the files hold, in a few plain sentences. */
export function summaryText(summary: LaserSummary, settings: LaserSettings): string {
  const sheets = summary.sheet_count === 1 ? "1 sheet" : `${count.format(summary.sheet_count)} sheets`;
  const sentences = [
    `${sheets} of ${settings.bed_width_mm} × ${settings.bed_height_mm} mm, ${count.format(summary.piece_count)} pieces, ${area.format(summary.material_m2)} m² of board.`,
  ];
  if (settings.has_base_plate) {
    const plate = `${Math.round(summary.plate_width_mm)} × ${Math.round(summary.plate_height_mm)} mm`;
    sentences.push(summary.plate_tiles > 1 ? `The base plate is ${plate}, larger than the bed, so it is cut in ${summary.plate_tiles} tiles.` : `The base plate is ${plate}.`);
  }
  if (settings.has_buildings) {
    sentences.push(`${plural(summary.building_count, "building")} in stacked layers.`);
    if (summary.dropped_buildings > 0) {
      const verb = summary.dropped_buildings === 1 ? "is" : "are";
      sentences.push(`${plural(summary.dropped_buildings, "building")} smaller than ${MIN_PIECE_MM} mm ${verb} left out, but still scored on the plate.`);
    }
    if (summary.estimated_heights > 0) {
      const verb = summary.estimated_heights === 1 ? "stands" : "stand";
      sentences.push(`${plural(summary.estimated_heights, "building")} without a measured height ${verb} ${DEFAULT_HEIGHT_M} m tall.`);
    }
  }

  return sentences.join(" ");
}

/** 1 -> "1 building", 1113 -> "1,113 buildings". */
function plural(value: number, noun: string): string {
  return value === 1 ? `1 ${noun}` : `${count.format(value)} ${noun}s`;
}

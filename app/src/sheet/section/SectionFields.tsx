// The section line in the inspector: how far beyond it the elevation reaches,
// its scale and how much taller heights are drawn, turning it round, and
// removing it. Drawing and reshaping happen on the sheet.

import type { Look } from "../../lib/types";
import { shortcutLabel } from "../../lib/platform";
import { Button } from "../../ui/Button";
import { NumberField } from "../../ui/NumberField";
import { Segmented } from "../../ui/Segmented";
import {
  EXAGGERATIONS,
  lengthOf,
  MAX_DEPTH_M,
  SECTION_SCALES,
  withoutSection,
  withSectionDepth,
  withSectionExaggeration,
  withSectionFlipped,
  withSectionScale,
} from "./sectionEdits";
import { SECTION_KEY } from "./useSectionTool";
import "./section.css";

const MIN_DEPTH_M = 5;
const DEPTH_STEP_M = 5;
// The select's value for "no scale chosen", since an <option> value is a string.
const FITTED = "fit";

type Props = {
  look: Look;
  // The depth, scale and heights only change the section view, so other views leave them out.
  isSectionView: boolean;
  onLook: (next: Look, mergeKey?: string) => void;
  onSeal: () => void;
};

export function SectionFields({ look, isSectionView, onLook, onSeal }: Props) {
  const line = look.section;
  if (!line) {
    return (
      <p className="section-hint">
        Draw a section line on the sheet with the section tool ({shortcutLabel(SECTION_KEY).toUpperCase()}) to cut a section.
      </p>
    );
  }

  return (
    <div className="field-stack">
      <span className="field-label">Section A–A′, {Math.round(lengthOf(line))} m long</span>
      {isSectionView && (
        <>
          <div className="field-stack">
            <span className="field-label">Seen beyond the line</span>
            <NumberField
              label="Seen beyond the line"
              value={line.depth_m}
              min={MIN_DEPTH_M}
              max={MAX_DEPTH_M}
              step={DEPTH_STEP_M}
              unit="m"
              onChange={(depth_m) => onLook(withSectionDepth(look, depth_m), "section-depth")}
              onSeal={onSeal}
            />
          </div>
          <label className="field-stack">
            <span className="field-label">Scale</span>
            <select
              className="text-input"
              value={line.scale ?? FITTED}
              onChange={(event) => {
                const value = event.target.value;
                onLook(withSectionScale(look, value === FITTED ? null : Number(value)));
              }}
            >
              <option value={FITTED}>Fit to the sheet</option>
              {SECTION_SCALES.map((scale) => (
                <option key={scale} value={scale}>
                  1:{scale}
                </option>
              ))}
            </select>
          </label>
          <div className="field-stack">
            <span className="field-label">Heights</span>
            <Segmented
              label="Heights"
              value={line.exaggeration ?? 1}
              options={EXAGGERATIONS.map((times) => ({
                value: times,
                label: `×${times}`,
                tip: times === 1 ? "Heights at the same scale as lengths" : `Heights drawn ${times} times taller; the legend says so`,
              }))}
              onChange={(times) => onLook(withSectionExaggeration(look, times))}
            />
          </div>
        </>
      )}
      <div className="section-actions">
        <Button icon="restore" tip="Look the other way: swap A and A′" onClick={() => onLook(withSectionFlipped(look))}>
          Flip
        </Button>
        <Button icon="trash" tip="Remove the section line" onClick={() => onLook(withoutSection(look))}>
          Remove
        </Button>
      </div>
    </div>
  );
}

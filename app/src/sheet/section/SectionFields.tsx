// The section line in the inspector: how far beyond it the elevation reaches,
// turning it round, and removing it. Drawing and reshaping happen on the sheet.

import type { Look } from "../../lib/types";
import { shortcutLabel } from "../../lib/platform";
import { Button } from "../../ui/Button";
import { NumberField } from "../../ui/NumberField";
import { lengthOf, MAX_DEPTH_M, withoutSection, withSectionDepth, withSectionFlipped } from "./sectionEdits";
import { SECTION_KEY } from "./useSectionTool";
import "./section.css";

const MIN_DEPTH_M = 5;
const DEPTH_STEP_M = 5;

type Props = {
  look: Look;
  onLook: (next: Look, mergeKey?: string) => void;
  onSeal: () => void;
};

export function SectionFields({ look, onLook, onSeal }: Props) {
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

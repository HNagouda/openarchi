import type { ReactNode } from "react";
import { formatScale } from "../../lib/format";
import type { Frame, Look, Paper } from "../../lib/types";
import { EditableText } from "../../ui/EditableText";
import { Icon } from "../../ui/Icon";
import { NumberField } from "../../ui/NumberField";
import { Section } from "../../ui/Section";
import { Segmented } from "../../ui/Segmented";
import { Toggle } from "../../ui/Toggle";
import { withSheet } from "./lookEdits";

const MIN_SCALE = 500;
const MAX_SCALE = 10_000;
const SCALE_STEP = 50;

const PAPERS: { value: Paper; label: string }[] = [
  { value: "a4", label: "A4" },
  { value: "a3", label: "A3" },
  { value: "a2", label: "A2" },
  { value: "a1", label: "A1" },
  { value: "a0", label: "A0" },
  { value: "square", label: "□" },
];

const FRAMES: { value: Frame; label: ReactNode; tip: string }[] = [
  { value: "circle", label: <Icon name="circle" size={14} />, tip: "Circle around the site" },
  { value: "square", label: <Icon name="square" size={14} />, tip: "Square" },
  { value: "area", label: <Icon name="area" size={14} />, tip: "The drawn area's own shape" },
];

type Props = { look: Look; onLook: (next: Look, mergeKey?: string) => void; onSeal: () => void };

/** Paper, scale, frame, titles and furniture. */
export function SheetSection({ look, onLook, onSeal }: Props) {
  const sheet = look.sheet;
  const isSquare = sheet.paper === "square";

  return (
    <Section id="sheet" title="Sheet">
      <div className="sheet-fields">
        <div className="title-fields">
          <EditableText
            className="sheet-title"
            value={sheet.title}
            placeholder="Add a title"
            allowEmpty
            onCommit={(title) => onLook(withSheet(look, { title }))}
          />
          <EditableText
            className="sheet-subtitle"
            value={sheet.subtitle}
            placeholder="Add a subtitle"
            allowEmpty
            onCommit={(subtitle) => onLook(withSheet(look, { subtitle }))}
          />
        </div>

        <div className="field-stack">
          <span className="field-label">Paper</span>
          <Segmented label="Paper" value={sheet.paper} options={PAPERS} onChange={(paper) => onLook(withSheet(look, { paper }))} />
        </div>
        <div className="field-pair">
          <div className="field-stack">
            <span className="field-label">Orientation</span>
            <Segmented
              label="Orientation"
              value={isSquare ? true : sheet.landscape}
              options={[
                { value: false, label: "Portrait", isDisabled: isSquare },
                { value: true, label: "Landscape", isDisabled: isSquare },
              ]}
              onChange={(landscape) => onLook(withSheet(look, { landscape }))}
            />
          </div>
          <div className="field-stack field-narrow">
            <span className="field-label">Frame</span>
            <Segmented label="Frame" value={sheet.frame} options={FRAMES} onChange={(frame) => onLook(withSheet(look, { frame }))} />
          </div>
        </div>
        <div className="field-stack">
          <span className="field-label">Scale</span>
          <NumberField
            label="Scale"
            value={sheet.scale}
            min={MIN_SCALE}
            max={MAX_SCALE}
            step={SCALE_STEP}
            isLogarithmic
            display={(value) => formatScale(value)}
            onChange={(scale) => onLook(withSheet(look, { scale }), "scale")}
            onSeal={onSeal}
          />
        </div>

        <div className="toggles">
          <Toggle label="Legend" isOn={sheet.show_legend} onChange={(show_legend) => onLook(withSheet(look, { show_legend }))} />
          <Toggle label="North arrow" isOn={sheet.show_north_arrow} onChange={(show_north_arrow) => onLook(withSheet(look, { show_north_arrow }))} />
          <Toggle label="Scale bar" isOn={sheet.show_scale_bar} onChange={(show_scale_bar) => onLook(withSheet(look, { show_scale_bar }))} />
          <Toggle label="Credits" isOn={sheet.show_credits} onChange={(show_credits) => onLook(withSheet(look, { show_credits }))} />
          <Toggle label="Neighbourhood" isOn={sheet.show_context} onChange={(show_context) => onLook(withSheet(look, { show_context }))} />
        </div>
      </div>
    </Section>
  );
}

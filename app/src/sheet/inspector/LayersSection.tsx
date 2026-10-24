import { useState } from "react";
import type { Catalog, Diagram, Layer, Look, Theme } from "../../lib/types";
import { Button } from "../../ui/Button";
import { ColorField } from "../../ui/ColorField";
import { Icon } from "../../ui/Icon";
import { NumberField } from "../../ui/NumberField";
import { Section } from "../../ui/Section";
import { swatchColour, withOverride } from "./lookEdits";

const MAX_STROKE_MM = 2;
const STROKE_STEP_MM = 0.05;
const PERCENT = 100;

type Props = {
  diagram: Diagram;
  theme: Theme;
  look: Look;
  labels: Catalog["layers"];
  layersPresent: Layer[];
  onLook: (next: Look, mergeKey?: string) => void;
  onSeal: () => void;
};

/** The diagram's layers, top first: visibility, and the user's own fill, stroke and opacity. */
export function LayersSection({ diagram, theme, look, labels, layersPresent, onLook, onSeal }: Props) {
  const [expanded, setExpanded] = useState<Layer | null>(null);
  const shown = [...diagram.layers]
    .reverse()
    .filter((layer) => layersPresent.length === 0 || layersPresent.includes(layer));
  const hasOverrides = Object.keys(look.overrides).length > 0;
  const labelOf = (layer: Layer) => labels.find((entry) => entry.id === layer)?.label ?? layer;

  return (
    <Section
      id="layers"
      title="Layers"
      aside={
        hasOverrides ? (
          <button type="button" className="text-button" onClick={() => onLook({ ...look, overrides: {} })}>
            Reset all
          </button>
        ) : undefined
      }
    >
      <ul className="layer-list">
        {shown.map((layer) => {
          const override = look.overrides[layer] ?? {};
          const style = theme.layers[layer];
          const isHidden = override.hidden === true;
          const isOpen = expanded === layer;
          const fill = override.fill ?? swatchColour(style?.fill, theme.palette);
          const stroke = override.stroke ?? swatchColour(style?.stroke, theme.palette);
          const isEdited = Object.keys(override).some((field) => field !== "hidden");

          return (
            <li key={layer} className={isOpen ? "layer is-open" : "layer"}>
              <div className={isHidden ? "layer-row is-hidden" : "layer-row"}>
                <Button
                  icon={isHidden ? "eyeOff" : "eye"}
                  tip={isHidden ? "Show layer" : "Hide layer"}
                  tipSide="left"
                  className="layer-eye"
                  onClick={() => onLook(withOverride(look, layer, { hidden: isHidden ? null : true }))}
                />
                <button type="button" className="layer-main" onClick={() => setExpanded(isOpen ? null : layer)} aria-expanded={isOpen}>
                  <span
                    className={fill ? "layer-swatch" : "layer-swatch is-empty"}
                    style={{ background: fill ?? undefined, boxShadow: stroke ? `inset 0 0 0 1.5px ${stroke}` : undefined }}
                  />
                  <span className="layer-name">{labelOf(layer)}</span>
                  {isEdited && <span className="layer-edited" aria-label="Edited" />}
                  {layer === diagram.emphasis && <span className="layer-badge">Focus</span>}
                  <span className="layer-chevron">
                    <Icon name="chevronDown" size={12} />
                  </span>
                </button>
              </div>
              {isOpen && (
                <div className="layer-detail">
                  <ColorField
                    label="Fill"
                    value={fill}
                    isOverridden={override.fill != null}
                    onChange={(hex) => onLook(withOverride(look, layer, { fill: hex }), `fill:${layer}`)}
                    onReset={() => onLook(withOverride(look, layer, { fill: null }))}
                  />
                  <ColorField
                    label="Stroke"
                    value={stroke}
                    isOverridden={override.stroke != null}
                    onChange={(hex) => onLook(withOverride(look, layer, { stroke: hex }), `stroke:${layer}`)}
                    onReset={() => onLook(withOverride(look, layer, { stroke: null }))}
                  />
                  <div className="detail-row">
                    <span className="field-label">Weight</span>
                    <NumberField
                      label="Stroke weight"
                      value={override.stroke_mm ?? style?.stroke_mm ?? 0}
                      min={0}
                      max={MAX_STROKE_MM}
                      step={STROKE_STEP_MM}
                      unit="mm"
                      display={(value) => value.toFixed(2)}
                      onChange={(value) => onLook(withOverride(look, layer, { stroke_mm: value }), `stroke_mm:${layer}`)}
                      onSeal={onSeal}
                    />
                  </div>
                  <div className="detail-row">
                    <span className="field-label">Opacity</span>
                    <NumberField
                      label="Opacity"
                      value={override.opacity ?? style?.opacity ?? 1}
                      min={0}
                      max={1}
                      step={0.01}
                      unit="%"
                      display={(value) => String(Math.round(value * PERCENT))}
                      read={(typed) => typed / PERCENT}
                      onChange={(value) => onLook(withOverride(look, layer, { opacity: value }), `opacity:${layer}`)}
                      onSeal={onSeal}
                    />
                  </div>
                </div>
              )}
            </li>
          );
        })}
      </ul>
    </Section>
  );
}

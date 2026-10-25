import { useState } from "react";
import type { Catalog, Diagram, Layer, Look, Theme } from "../../lib/types";
import { Button } from "../../ui/Button";
import { ColorField } from "../../ui/ColorField";
import { Icon } from "../../ui/Icon";
import { NumberField } from "../../ui/NumberField";
import { Section } from "../../ui/Section";
import { mixHex, swatchColour, withOverride } from "./lookEdits";

const MAX_STROKE_MM = 2;
const STROKE_STEP_MM = 0.05;
const PERCENT = 100;
/** How far a blank layer's fill leans from paper towards ink, as BLANK_TINT in crates/core/src/render/style.rs. */
const BLANK_TINT = 0.08;
/** All a section draws, whatever the diagram: SECTION_LAYERS in crates/core/src/render/mod.rs. */
const SECTION_LAYERS: Layer[] = ["building", "tree"];

type Props = {
  /** What the sheet draws: one diagram, or the explode plates top first (`sheetDiagrams`). */
  diagrams: Diagram[];
  isSection: boolean;
  theme: Theme;
  look: Look;
  labels: Catalog["layers"];
  layersPresent: Layer[];
  onLook: (next: Look, mergeKey?: string) => void;
  onSeal: () => void;
};

/** The drawn layers, top first: visibility, and the user's own fill, stroke and opacity. */
export function LayersSection({ diagrams, isSection, theme, look, labels, layersPresent, onLook, onSeal }: Props) {
  const [expanded, setExpanded] = useState<Layer | null>(null);
  const drawn = new Set(diagrams.flatMap((diagram) => [...diagram.layers].reverse()));
  const shown = [...drawn]
    .filter((layer) => !isSection || SECTION_LAYERS.includes(layer))
    .filter((layer) => layersPresent.length === 0 || layersPresent.includes(layer));
  // Blank only where every diagram drawing the layer draws it blank, so the swatch matches the sheet.
  const isBlankLayer = (layer: Layer) =>
    diagrams.every((diagram) => !diagram.layers.includes(layer) || (diagram.blank?.includes(layer) ?? false));
  const hasOverrides = Object.keys(look.overrides).length > 0;
  const labelOf = (layer: Layer) => labels.find((entry) => entry.id === layer)?.label ?? layer;

  return (
    <Section
      id="layers"
      title="Layers"
      aside={
        hasOverrides ? (
          <Button variant="ghost" onClick={() => onLook({ ...look, overrides: {} })}>
            Reset all
          </Button>
        ) : undefined
      }
    >
      <ul className="layer-list">
        {shown.map((layer) => {
          const override = look.overrides[layer] ?? {};
          const style = theme.layers[layer];
          const isHidden = override.hidden === true;
          const isOpen = expanded === layer;
          const isBlank = isBlankLayer(layer);
          const { paper, ink } = theme.palette;
          const drawnFill = isBlank ? mixHex(paper, ink, BLANK_TINT) : swatchColour(style?.fill, theme.palette);
          const drawnStroke = isBlank ? ink : swatchColour(style?.stroke, theme.palette);
          const fill = override.fill ?? drawnFill;
          const stroke = override.stroke ?? drawnStroke;
          const isEdited = Object.keys(override).some((field) => field !== "hidden");

          return (
            <li key={layer} className={isOpen ? "layer is-open" : "layer"}>
              <div className={isHidden ? "layer-row is-hidden" : "layer-row"}>
                <button
                  type="button"
                  className="layer-main"
                  onClick={() => setExpanded(isOpen ? null : layer)}
                  aria-expanded={isOpen}
                  data-tip={isOpen ? undefined : "Fill, stroke and opacity"}
                  data-tip-side="above"
                >
                  {/* The same disclosure chevron as the section header, always shown, so the row reads as something that opens. */}
                  <span className="layer-chevron">
                    <Icon name="chevronRight" size={12} />
                  </span>
                  <span
                    className={fill ? "layer-swatch" : "layer-swatch is-empty"}
                    style={{ background: fill ?? undefined, boxShadow: stroke ? `inset 0 0 0 1.5px ${stroke}` : undefined }}
                  />
                  <span className="layer-name">{labelOf(layer)}</span>
                  {isEdited && <span className="layer-edited" data-tip="Changed from the theme" data-tip-side="left" />}
                  {diagrams.some((diagram) => diagram.emphasis === layer) && <span className="layer-badge">Focus</span>}
                </button>
                <Button
                  icon={isHidden ? "eyeOff" : "eye"}
                  tip={isHidden ? "Show layer" : "Hide layer"}
                  tipSide="left"
                  className={isHidden ? "layer-eye is-off" : "layer-eye"}
                  onClick={() => onLook(withOverride(look, layer, { hidden: isHidden ? null : true }))}
                />
              </div>
              {isOpen && (
                <div className="layer-detail">
                  <ColorField
                    label="Fill"
                    value={fill}
                    isOverridden={override.fill != null}
                    onChange={(hex) => onLook(withOverride(look, layer, { fill: hex }), `fill:${layer}`)}
                    onReset={() => onLook(withOverride(look, layer, { fill: null }))}
                    onSeal={onSeal}
                  />
                  <ColorField
                    label="Stroke"
                    value={stroke}
                    isOverridden={override.stroke != null}
                    onChange={(hex) => onLook(withOverride(look, layer, { stroke: hex }), `stroke:${layer}`)}
                    onReset={() => onLook(withOverride(look, layer, { stroke: null }))}
                    onSeal={onSeal}
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

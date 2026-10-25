// The formatting bar over the selected text: family, size, bold, italic,
// alignment and colour, plus putting a moved title back and deleting a box.
// It shows the style as the core resolved it, so "theme" values read as what
// is actually printed.

import { useContext } from "react";
import { snapToStep } from "../../lib/format";
import type { Align, TextStyle } from "../../lib/types";
import { Button } from "../../ui/Button";
import { ColorPicker, PaletteSwatches } from "../../ui/ColorPicker";
import { ColorSwatch } from "../../ui/ColorSwatch";
import { NumberBox } from "../../ui/NumberField";
import type { SheetText } from "./sheetTexts";

// The bundled families, in one place for the menu. The core refuses any other.
const FAMILIES = ["Inter", "Source Serif 4", "IBM Plex Mono"];
// The same limits the core enforces.
const MIN_SIZE_PT = 2;
const MAX_SIZE_PT = 400;
// Sizes are shown and typed to a tenth of a point.
const SIZE_STEP_PT = 0.1;
// A nearly clear text still shows its colour on the swatch.
const MIN_SWATCH_OPACITY = 0.25;

const ALIGNMENTS: { value: Align; icon: "alignLeft" | "alignCentre" | "alignRight"; tip: string }[] = [
  { value: "left", icon: "alignLeft", tip: "Align left" },
  { value: "centre", icon: "alignCentre", tip: "Centre" },
  { value: "right", icon: "alignRight", tip: "Align right" },
];

type Props = {
  text: SheetText;
  style: TextStyle;
  isMoved: boolean;
  isBox: boolean;
  onStyle: (patch: TextStyle, mergeKey?: string) => void;
  onSeal: () => void;
  onBackToLayout: () => void;
  onDelete: () => void;
};

export function TextToolbar({ text, style, isMoved, isBox, onStyle, onSeal, onBackToLayout, onDelete }: Props) {
  const palette = useContext(PaletteSwatches);

  return (
    <div className="text-toolbar" role="toolbar" aria-label="Text" onPointerDown={(event) => event.stopPropagation()} onDoubleClick={(event) => event.stopPropagation()}>
      <select
        className="text-family"
        aria-label="Font"
        value={style.family ?? ""}
        onChange={(event) => onStyle({ family: event.target.value === "" ? null : event.target.value })}
      >
        <option value="">Palette ({text.family})</option>
        {FAMILIES.map((family) => (
          <option key={family} value={family}>
            {family}
          </option>
        ))}
      </select>
      <NumberBox
        className="text-size"
        value={snapToStep(text.sizePt, SIZE_STEP_PT)}
        min={MIN_SIZE_PT}
        max={MAX_SIZE_PT}
        step={SIZE_STEP_PT}
        unit="pt"
        label="Size in points"
        tip="Size in points"
        onChange={(size_pt) => onStyle({ size_pt })}
      />
      <span className="toolbar-divider" />
      <Button className="text-bold" tip="Bold" isActive={text.isBold} aria-pressed={text.isBold} onClick={() => onStyle({ bold: !text.isBold })}>
        B
      </Button>
      <Button className="text-italic" tip="Italic" isActive={text.isItalic} aria-pressed={text.isItalic} onClick={() => onStyle({ italic: !text.isItalic })}>
        I
      </Button>
      <span className="toolbar-divider" />
      {ALIGNMENTS.map((alignment) => (
        <Button
          key={alignment.value}
          icon={alignment.icon}
          tip={alignment.tip}
          isActive={text.align === alignment.value}
          aria-pressed={text.align === alignment.value}
          onClick={() => onStyle({ align: alignment.value })}
        />
      ))}
      <span className="toolbar-divider" />
      <ColorSwatch
        className="text-colour"
        colour={text.colour}
        style={{ opacity: Math.max(text.opacity, MIN_SWATCH_OPACITY) }}
        label="Text colour"
        tip="Colour"
        onSeal={onSeal}
      >
        <ColorPicker
          value={text.colour}
          acceptsRoles
          selected={style.colour ?? null}
          swatches={palette}
          opacity={text.opacity}
          onOpacity={(opacity) => onStyle({ opacity }, `opacity:${text.id}`)}
          onChange={(colour) => onStyle({ colour }, `colour:${text.id}`)}
        />
      </ColorSwatch>
      {(isMoved && !isBox) && (
        <Button icon="reset" tip="Back to the automatic layout" onClick={onBackToLayout} />
      )}
      {isBox && <Button icon="trash" tip="Delete" shortcut="delete" onClick={onDelete} />}
    </div>
  );
}

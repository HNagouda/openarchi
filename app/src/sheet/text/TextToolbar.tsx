// The formatting bar over the selected text: family, size, bold, italic,
// alignment and colour, plus putting a moved title back and deleting a box.
// It shows the style as the core resolved it, so "theme" values read as what
// is actually printed.

import { useContext, useEffect, useRef, useState } from "react";
import type { Align, TextStyle } from "../../lib/types";
import { Button } from "../../ui/Button";
import { ColorPicker, PaletteSwatches } from "../../ui/ColorPicker";
import { ColorPopover } from "../../ui/ColorPopover";
import { rememberColour } from "../../ui/colour";
import type { SheetText } from "./sheetTexts";

// The bundled families, in one place for the menu. The core refuses any other.
const FAMILIES = ["Inter", "Source Serif 4", "IBM Plex Mono"];
// The same limits the core enforces.
const MIN_SIZE_PT = 2;
const MAX_SIZE_PT = 400;

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
  const [isColourOpen, setColourOpen] = useState(false);
  const swatch = useRef<HTMLButtonElement>(null);
  const palette = useContext(PaletteSwatches);

  const closeColour = () => {
    setColourOpen(false);
    rememberColour(text.colour);
    onSeal();
  };

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
      <SizeField sizePt={text.sizePt} onCommit={(size_pt) => onStyle({ size_pt })} />
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
      <button
        ref={swatch}
        type="button"
        className="swatch text-colour"
        style={{ background: text.colour, opacity: Math.max(text.opacity, 0.25) }}
        data-tip="Colour"
        aria-label="Text colour"
        aria-expanded={isColourOpen}
        onClick={() => (isColourOpen ? closeColour() : setColourOpen(true))}
      />
      {isColourOpen && (
        <ColorPopover anchor={swatch.current} label="Text colour" onClose={closeColour}>
          <ColorPicker
            value={text.colour}
            acceptsRoles
            selected={style.colour ?? null}
            swatches={palette}
            opacity={text.opacity}
            onOpacity={(opacity) => onStyle({ opacity }, `opacity:${text.id}`)}
            onChange={(colour) => onStyle({ colour }, `colour:${text.id}`)}
          />
        </ColorPopover>
      )}
      {(isMoved && !isBox) && (
        <Button icon="reset" tip="Back to the automatic layout" onClick={onBackToLayout} />
      )}
      {isBox && <Button icon="trash" tip="Delete" shortcut="delete" onClick={onDelete} />}
    </div>
  );
}

/** Points, typed; Return or leaving the field applies them. */
function SizeField({ sizePt, onCommit }: { sizePt: number; onCommit: (sizePt: number) => void }) {
  const shown = String(Math.round(sizePt * 10) / 10);
  const [draft, setDraft] = useState(shown);

  useEffect(() => setDraft(shown), [shown]);

  const commit = () => {
    const typed = Number(draft.replace(",", "."));
    if (draft.trim() === "" || Number.isNaN(typed)) {
      setDraft(shown);
      return;
    }
    const clamped = Math.min(MAX_SIZE_PT, Math.max(MIN_SIZE_PT, typed));
    if (String(clamped) !== shown) onCommit(clamped);
    setDraft(String(clamped));
  };

  return (
    <label className="text-size number-box" data-tip="Size in points">
      <input
        className="number-input tabular"
        inputMode="decimal"
        aria-label="Size in points"
        value={draft}
        onChange={(event) => setDraft(event.target.value)}
        onBlur={commit}
        onKeyDown={(event) => {
          if (event.key === "Enter") commit();
          if (event.key === "Escape") setDraft(shown);
        }}
      />
      <span className="number-unit">pt</span>
    </label>
  );
}
